//! Config loader.
//!
//! Defaults are baked into the binary at compile time via `include_str!`.
//! Optional on-disk `config/application.toml` overrides them. Environment
//! variables prefixed with `NETLAB_` and separated by `__` take the
//! highest priority (e.g. `NETLAB_SERVER__PORT=9073`).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NetlabConfig {
    pub server: ServerConfig,
    pub port: PortPoolConfig,
    pub ssl: SslConfig,
    pub metrics: MetricsConfig,
    pub app: AppConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub static_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PortPoolConfig {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SslConfig {
    /// Path to a PEM file containing the leaf cert + intermediates
    /// (a.k.a. `fullchain.pem` from Let's Encrypt / `caddy`).
    #[serde(default)]
    pub cert_path: Option<String>,
    /// Path to a PEM file containing the private key (PKCS#8 or RSA).
    #[serde(default)]
    pub key_path: Option<String>,
    /// Optional password for an encrypted PEM private key.
    #[serde(default)]
    pub key_password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
}

impl NetlabConfig {
    pub fn load() -> anyhow::Result<Self> {
        // 1. Start with the embedded defaults.
        const EMBEDDED_TOML: &str = include_str!("../config/application.toml");
        let mut value: toml::Value =
            toml::from_str(EMBEDDED_TOML).context("parsing embedded config")?;

        // 2. Overlay the on-disk file (if present).
        for path in ["config/application.toml", "./application.toml"] {
            if let Ok(s) = std::fs::read_to_string(path) {
                let overlay: toml::Value =
                    toml::from_str(&s).with_context(|| format!("parsing {path}"))?;
                merge_toml(&mut value, overlay);
                tracing::info!("loaded config override from {path}");
                break;
            }
        }

        // 3. Overlay env vars (NETLAB_SERVER__PORT -> server.port etc.).
        apply_env_overrides(&mut value);

        // Debug: dump the merged value to a file we can inspect.
        if let Ok(s) = toml::to_string_pretty(&value) {
            let _ = std::fs::write("merged-config-debug.toml", s);
        }

        // 4. Deserialize.
        let cfg: NetlabConfig = value.try_into().context("deserializing config")?;
        Ok(cfg)
    }
}

fn merge_toml(base: &mut toml::Value, overlay: toml::Value) {
    use toml::Value;
    if let Value::Table(base_t) = base {
        if let Value::Table(overlay_t) = overlay {
            for (k, v) in overlay_t {
                match base_t.get_mut(&k) {
                    Some(existing @ Value::Table(_)) => {
                        if matches!(v, Value::Table(_)) {
                            merge_toml(existing, v);
                        } else {
                            base_t.insert(k, v);
                        }
                    }
                    _ => {
                        base_t.insert(k, v);
                    }
                }
            }
        } else {
            *base = overlay;
        }
    } else {
        *base = overlay;
    }
}

fn apply_env_overrides(value: &mut toml::Value) {
    use toml::Value;
    for (key, val) in std::env::vars() {
        let Some(rest) = key.strip_prefix("NETLAB_") else {
            continue;
        };
        let path: Vec<&str> = rest.split("__").collect();
        if path.is_empty() {
            continue;
        }
        insert_at_path(value, &path, parse_env_value(&val));
    }
}

fn parse_env_value(s: &str) -> toml::Value {
    // Try bool, int, float, fall back to string.
    if s.eq_ignore_ascii_case("true") {
        return toml::Value::Boolean(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return toml::Value::Boolean(false);
    }
    if let Ok(n) = s.parse::<i64>() {
        return toml::Value::Integer(n);
    }
    if let Ok(n) = s.parse::<f64>() {
        return toml::Value::Float(n);
    }
    toml::Value::String(s.to_string())
}

fn insert_at_path(value: &mut toml::Value, path: &[&str], new: toml::Value) {
    use toml::Value;
    if path.is_empty() {
        return;
    }
    if path.len() == 1 {
        if let Value::Table(t) = value {
            t.insert(path[0].to_string(), new);
        }
        return;
    }
    if let Value::Table(t) = value {
        let entry = t
            .entry(path[0].to_string())
            .or_insert_with(|| Value::Table(toml::map::Map::new()));
        if !matches!(entry, Value::Table(_)) {
            *entry = Value::Table(toml::map::Map::new());
        }
        insert_at_path(entry, &path[1..], new);
    }
}

use anyhow::Context;
