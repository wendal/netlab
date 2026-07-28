//! Hex codec helpers used by the WS framing layer.
//!
//! These are intentionally thin wrappers around the [`hex`] crate — they
//! translate decode failures into [`AppError::BadHex`] so callers can
//! bubble errors with `?` without ever needing to `unwrap`.

use crate::domain::errors::AppError;

/// Encode a byte slice as a lowercase hex string.
pub fn encode_bytes_to_hex(b: &[u8]) -> String {
    hex::encode(b)
}

/// Decode a hex string into raw bytes.
///
/// Returns [`AppError::BadHex`] when the input is not a valid hex sequence
/// (wrong length, non-hex character, etc.).
pub fn decode_hex_to_bytes(s: &str) -> Result<Vec<u8>, AppError> {
    hex::decode(s).map_err(|_| AppError::BadHex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_short() {
        let bytes = b"hello";
        let encoded = encode_bytes_to_hex(bytes);
        assert_eq!(encoded, "68656c6c6f");
        let decoded = decode_hex_to_bytes(&encoded).expect("decode");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn roundtrip_empty() {
        let encoded = encode_bytes_to_hex(b"");
        assert_eq!(encoded, "");
        let decoded = decode_hex_to_bytes(&encoded).expect("decode empty");
        assert!(decoded.is_empty());
    }

    #[test]
    fn decode_bad_hex_returns_bad_hex_error() {
        // "zz" is not a valid hex pair
        let err = decode_hex_to_bytes("zz").expect_err("should fail");
        assert!(matches!(err, AppError::BadHex));
    }

    #[test]
    fn decode_odd_length_returns_bad_hex_error() {
        // An odd number of hex chars is invalid
        let err = decode_hex_to_bytes("abc").expect_err("odd length should fail");
        assert!(matches!(err, AppError::BadHex));
    }

    #[test]
    fn stress_one_mib_roundtrip() {
        // Build a deterministic 1 MiB payload so the encode/decode paths
        // are exercised at non-trivial scale.
        const SIZE: usize = 1024 * 1024;
        let mut payload = Vec::with_capacity(SIZE);
        for i in 0..SIZE {
            // Mix of values across the byte range
            payload.push((i % 251) as u8);
        }
        assert_eq!(payload.len(), SIZE);

        let encoded = encode_bytes_to_hex(&payload);
        assert_eq!(encoded.len(), SIZE * 2);

        let decoded = decode_hex_to_bytes(&encoded).expect("decode 1 MiB");
        assert_eq!(decoded.len(), SIZE);
        assert_eq!(decoded, payload);
    }
}