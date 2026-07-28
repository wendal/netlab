//! TCP "protocol" parsing strategies.
//!
//! The default [`DumpProtocol`] is a transparent pass-through: every chunk
//! the kernel hands us is forwarded as-is. This mirrors the Java
//! `SimpleTcpDumpProtocol` (来什么都直接读取出来) from the original
//! `luatos-netlab` server.
//!
//! Keeping the abstraction in place (rather than inlining `read()` into
//! the entity) leaves room for future framing formats -- e.g. length-prefix
//! or COBS -- without touching the connection lifecycle.

use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::domain::errors::AppError;

/// Strategy for turning a raw TCP byte stream into outbound data chunks.
///
/// Implementations are stateless and may be shared across every connection
/// served by a single [`TcpPortEntity`](super::entity::TcpPortEntity).
#[async_trait::async_trait]
pub trait TcpProtocol: Send + Sync {
    /// Read one chunk from `reader`. Returns `(bytes, finished)` where
    /// `finished` is `true` when the peer closed the connection (EOF).
    ///
    /// The reader is passed as a boxed `Pin<&mut dyn AsyncRead>` so the
    /// trait stays object-safe -- the caller (a connection task) owns the
    /// concrete stream type and only hands the trait object during read.
    async fn read_chunk(
        &self,
        reader: &mut Pin<Box<dyn AsyncRead + Send>>,
    ) -> Result<(Vec<u8>, bool), AppError>;
}

/// Plain pass-through: emit whatever the OS hands us, up to 16 KiB per read.
pub struct DumpProtocol;

#[async_trait::async_trait]
impl TcpProtocol for DumpProtocol {
    async fn read_chunk(
        &self,
        reader: &mut Pin<Box<dyn AsyncRead + Send>>,
    ) -> Result<(Vec<u8>, bool), AppError> {
        let mut buf = vec![0u8; 16 * 1024];
        let n = reader.read(&mut buf).await.map_err(AppError::Io)?;
        buf.truncate(n);
        Ok((buf, n == 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn dump_protocol_returns_bytes_until_eof() {
        let payload: Vec<u8> = (0..200u8).collect();
        let mut reader: Pin<Box<dyn AsyncRead + Send>> =
            Box::pin(BufReader::new(Cursor::new(payload.clone())));
        let proto = DumpProtocol;

        let (bytes, finished) = proto.read_chunk(&mut reader).await.expect("first read");
        assert!(!finished);
        assert_eq!(bytes.len(), 200);
        assert_eq!(bytes, payload);

        let (bytes, finished) = proto.read_chunk(&mut reader).await.expect("second read");
        assert!(finished);
        assert!(bytes.is_empty());
    }

    #[tokio::test]
    async fn dump_protocol_handles_empty_input() {
        let mut reader: Pin<Box<dyn AsyncRead + Send>> =
            Box::pin(BufReader::new(Cursor::new(Vec::<u8>::new())));
        let proto = DumpProtocol;
        let (bytes, finished) = proto.read_chunk(&mut reader).await.expect("read");
        assert!(finished);
        assert!(bytes.is_empty());
    }
}
