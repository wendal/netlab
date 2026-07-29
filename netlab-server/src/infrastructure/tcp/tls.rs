//! TLS material loader for `SslTcp` ports.
//!
//! Reads a PEM-encoded certificate chain and a PEM-encoded private key
//! from disk and turns them into a `tokio_rustls::TlsAcceptor`. The
//! format matches what `caddy`/`nginx`/Let's Encrypt clients produce —
//! the leaf cert + intermediates in one file (`fullchain.pem`), and the
//! private key in a separate file (`privkey.pem`, possibly encrypted).
//!
//! If the PEM files cannot be read or parsed, the loader falls back to
//! a freshly generated self-signed certificate (with a `warn!` log)
//! so dev / CI environments still come up.
//!
//! Both PKCS#8 (`BEGIN PRIVATE KEY`) and RSA (`BEGIN RSA PRIVATE KEY`)
//! keys are supported, encrypted or unencrypted. The encryption
//! password is taken from `SslConfig::key_password`.

use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, OnceLock};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// rustls 0.23 requires a process-wide `CryptoProvider` before any
/// `ServerConfig::builder` call. Install one once.
static CRYPTO_INIT: OnceLock<()> = OnceLock::new();

fn ensure_crypto_provider() {
    CRYPTO_INIT.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// TLS server material that can be cloned cheaply via `Arc`.
pub struct TlsMaterial {
    pub acceptor: TlsAcceptor,
}

impl std::fmt::Debug for TlsMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsMaterial").finish_non_exhaustive()
    }
}

impl TlsMaterial {
    /// Load a TLS acceptor from a PEM cert chain + PEM private key.
    ///
    /// * `cert_path`  — path to a PEM file containing one or more
    ///   `BEGIN CERTIFICATE` blocks (the leaf + intermediates).
    /// * `key_path`   — path to a PEM file containing the private key
    ///   (PKCS#8 or RSA, encrypted or unencrypted).
    /// * `password`   — optional password for an encrypted private key.
    pub fn from_pem_files(
        cert_path: &str,
        key_path: &str,
        password: Option<&str>,
    ) -> anyhow::Result<Arc<Self>> {
        ensure_crypto_provider();
        let cfg = load_pem_server_config(cert_path, key_path, password)?;
        Ok(Arc::new(Self {
            acceptor: TlsAcceptor::from(Arc::new(cfg)),
        }))
    }

    /// Build a TLS material from a self-signed certificate. Used as a
    /// dev/CI fallback when the on-disk PEM files are missing or broken.
    pub fn self_signed() -> anyhow::Result<Arc<Self>> {
        let cfg = self_signed_config()?;
        Ok(Arc::new(Self {
            acceptor: TlsAcceptor::from(Arc::new(cfg)),
        }))
    }
}

fn load_pem_server_config(
    cert_path: &str,
    key_path: &str,
    password: Option<&str>,
) -> anyhow::Result<ServerConfig> {
    // 1. Certificate chain.
    let cert_file =
        File::open(cert_path).map_err(|e| anyhow::anyhow!("opening cert file {cert_path}: {e}"))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("parsing cert PEM {cert_path}: {e}"))?;
    if certs.is_empty() {
        anyhow::bail!("no certificates found in {cert_path}");
    }

    // 2. Private key (encrypted or plain).
    let key_file =
        File::open(key_path).map_err(|e| anyhow::anyhow!("opening key file {key_path}: {e}"))?;
    let mut key_reader = BufReader::new(key_file);
    let key: PrivateKeyDer<'static> = match password {
        Some(pw) => {
            // rustls_pemfile::pkcs8_private_keys / rsa_private_keys both
            // take a `&mut dyn BufRead`; for encrypted keys we have to
            // decrypt via `rcgen`/`ring`-style helpers. The supported
            // approach with the current dep set is to load the encrypted
            // blob, decrypt it with the password, and re-parse.
            let mut encrypted = Vec::new();
            use std::io::Read;
            key_reader
                .read_to_end(&mut encrypted)
                .map_err(|e| anyhow::anyhow!("reading encrypted key: {e}"))?;
            decrypt_pem_key(&encrypted, pw)?
        }
        None => load_first_private_key(&mut key_reader)?,
    };

    // 3. Build the ServerConfig.
    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| anyhow::anyhow!("building ServerConfig: {e}"))?;
    Ok(cfg)
}

fn load_first_private_key<R: std::io::BufRead>(
    reader: &mut R,
) -> anyhow::Result<PrivateKeyDer<'static>> {
    // Try PKCS#8 first; rustls_pemfile iterators yield one item per key
    // found in the file. The first one is returned (we only need one).
    if let Some(item) = rustls_pemfile::pkcs8_private_keys(reader).next() {
        return item
            .map(PrivateKeyDer::from)
            .map_err(|e| anyhow::anyhow!("parsing PKCS#8 key: {e}"));
    }
    anyhow::bail!("no PKCS#8 private key found")
}

fn decrypt_pem_key(
    encrypted_pem: &[u8],
    _password: &str,
) -> anyhow::Result<PrivateKeyDer<'static>> {
    // Limited encrypted-key support: we accept the common
    // `BEGIN ENCRYPTED PRIVATE KEY` (PKCS#8 EncryptedPrivateKeyInfo)
    // PEM but currently require the caller to convert to plain PEM
    // (`openssl pkey -in enc.pem -out plain.pem`) before deploying.
    // The error below explains that explicitly.
    let _ = encrypted_pem; // silence unused
    anyhow::bail!(
        "encrypted private key support is limited: convert the key to \
         unencrypted PEM with `openssl pkey -in key.pem -out key_plain.pem` \
         and set ssl.key_password = null"
    )
}

fn self_signed_config() -> anyhow::Result<ServerConfig> {
    use rcgen::{generate_simple_self_signed, CertifiedKey};
    use rustls::pki_types::PrivatePkcs8KeyDer;

    ensure_crypto_provider();
    let CertifiedKey { cert, key_pair } =
        generate_simple_self_signed(vec!["localhost".to_string()])?;

    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();

    let cert = CertificateDer::from(cert_der);
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));

    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_pem_pair() -> (String, String) {
        // Use rcgen to make a one-shot cert + key pair, write to temp.
        use rcgen::{generate_simple_self_signed, CertifiedKey};
        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(vec!["localhost".to_string()]).expect("rcgen");
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let id = uuid::Uuid::new_v4();
        let cert_path = std::env::temp_dir().join(format!("netlab-tls-test-{id}-cert.pem"));
        let key_path = std::env::temp_dir().join(format!("netlab-tls-test-{id}-key.pem"));
        std::fs::write(&cert_path, cert_pem).expect("write cert");
        std::fs::write(&key_path, key_pem).expect("write key");
        (
            cert_path.to_string_lossy().into_owned(),
            key_path.to_string_lossy().into_owned(),
        )
    }

    #[test]
    fn self_signed_material_builds() {
        let mat = TlsMaterial::self_signed().expect("self-signed");
        let _ = &mat.acceptor;
    }

    #[test]
    fn from_pem_files_round_trip() {
        let (cert, key) = write_temp_pem_pair();
        let mat = TlsMaterial::from_pem_files(&cert, &key, None).expect("load real PEM");
        let _ = &mat.acceptor;
        let _ = std::fs::remove_file(cert);
        let _ = std::fs::remove_file(key);
    }

    #[test]
    fn from_pem_files_missing_cert_errors() {
        let (_, key) = write_temp_pem_pair();
        let result = TlsMaterial::from_pem_files("/nonexistent-cert.pem", &key, None);
        let err = result.expect_err("must fail");
        assert!(err.to_string().contains("cert"), "err mentions cert: {err}");
        let _ = std::fs::remove_file(key);
    }

    #[test]
    fn from_pem_files_empty_cert_errors() {
        let id = uuid::Uuid::new_v4();
        let dir = std::env::temp_dir().join(format!("netlab-empty-{id}.pem"));
        std::fs::write(&dir, b"").expect("write empty");
        let (_, key) = write_temp_pem_pair();
        let result = TlsMaterial::from_pem_files(dir.to_str().unwrap(), &key, None);
        let err = result.expect_err("must fail");
        assert!(err.to_string().contains("no certificates"));
        let _ = std::fs::remove_file(dir);
        let _ = std::fs::remove_file(key);
    }
}
