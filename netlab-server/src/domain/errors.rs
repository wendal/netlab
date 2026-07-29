//! Domain error types.
//!
//! All errors in the domain layer funnel through [`AppError`]. Variants are
//! intentionally narrow and free of IO; concrete IO errors are wrapped via
//! the [`AppError::Io`] and [`AppError::PortBind`] variants so the domain
//! itself stays pure.

use std::fmt;
use std::io;

/// All domain-level errors. Implementations of `From` exist for the common
/// external error types (`serde_json::Error`, `io::Error`) so that
/// `?` works without ever needing to `unwrap`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The configured port pool has been exhausted.
    #[error("no port available in pool")]
    NoPortAvailable,

    /// The wire-level `type` field could not be parsed into a known variant.
    #[error("bad port type: {0}")]
    BadType(String),

    /// A hex string could not be decoded.
    #[error("bad hex input")]
    BadHex,

    /// Binding a socket failed at the OS level; carries the underlying message.
    #[error("port bind failed: {0}")]
    PortBind(String),

    /// The requested port is already bound by something else.
    #[error("port {0} already in use")]
    PortInUse(u16),

    /// A client id could not be found in any registry.
    #[error("unknown client: {0}")]
    UnknownClient(String),

    /// The caller supplied a token that does not match the port's token.
    #[error("bad token")]
    BadToken,

    /// JSON (de)serialisation failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic IO failure (read/write/socket etc.).
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

// Compile-time guarantee that the variants `Display` for `AppError`:
// the `thiserror::Error` derive already gives us `Display`, but we add
// an explicit `fmt::Display` impl check here so the trait bounds are
// obvious to readers.
const _: fn() = || {
    fn assert_display<T: fmt::Display>() {}
    assert_display::<AppError>();
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_text_for_simple_variants() {
        assert_eq!(
            AppError::NoPortAvailable.to_string(),
            "no port available in pool"
        );
        assert_eq!(
            AppError::BadType("xxx".into()).to_string(),
            "bad port type: xxx"
        );
        assert_eq!(AppError::BadHex.to_string(), "bad hex input");
        assert_eq!(
            AppError::PortBind("os".into()).to_string(),
            "port bind failed: os"
        );
        assert_eq!(
            AppError::PortInUse(8080).to_string(),
            "port 8080 already in use"
        );
        assert_eq!(
            AppError::UnknownClient("abc".into()).to_string(),
            "unknown client: abc"
        );
        assert_eq!(AppError::BadToken.to_string(), "bad token");
    }

    #[test]
    fn from_io_error_preserves_kind_message() {
        let io_err = io::Error::other("boom");
        let err: AppError = io_err.into();
        match err {
            AppError::Io(e) => assert_eq!(e.to_string(), "boom"),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn from_serde_json_error_routes_to_json_variant() {
        // Force a serde_json error by feeding garbage to from_str.
        let bad: serde_json::Error = serde_json::from_str::<i32>("not-a-number")
            .expect_err("serde should fail");
        let err: AppError = bad.into();
        assert!(matches!(err, AppError::Json(_)));
    }

    #[test]
    fn debug_does_not_panic() {
        // Just exercise Debug for every variant.
        let samples = [
            AppError::NoPortAvailable,
            AppError::BadType("x".into()),
            AppError::BadHex,
            AppError::PortBind("x".into()),
            AppError::PortInUse(1),
            AppError::UnknownClient("x".into()),
            AppError::BadToken,
        ];
        for s in samples {
            let _ = format!("{s:?}");
        }
    }
}
