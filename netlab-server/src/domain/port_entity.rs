//! Domain abstractions for a bound port and its wire-level events.
//!
//! `PortEntity` is the trait the application layer implements for each
//! transport (TCP, TCP+SSL, UDP). The domain layer only declares the
//! interface and the wire-protocol enums — concrete implementations
//! live in `application/` and `infrastructure/`.

use crate::domain::client::ClientId;
use crate::domain::errors::AppError;
use crate::domain::port::PortNumber;

/// Events emitted from the network layer towards the WebSocket peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsEvent {
    /// A new client connected on the port.
    Connected {
        client: ClientId,
        addr: String,
    },
    /// Data arrived from a connected client.
    Data {
        client: ClientId,
        data: String,
        /// `true` if `data` is hex-encoded; `false` for plain text.
        hex: bool,
    },
    /// A client disconnected.
    Closed { client: ClientId },
    /// An error occurred on this port.
    Error { msg: String },
}

/// Internal transport classification used by the application layer.
///
/// Distinct from [`PortType`]: `PortKind` enumerates the *runtime*
/// implementation (raw TCP / TLS / UDP), while `PortType` describes
/// the wire-protocol label seen by WebSocket clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortKind {
    Tcp,
    TcpSsl,
    Udp,
}

/// Wire-protocol `type` label exchanged with WebSocket clients.
///
/// Mapping rules:
/// * `"tcp"`       -> `Tcp`
/// * `"udp"`       -> `Udp`
/// * `"tcp_ssl"` /
///   `"ssl-tcp"`   -> `SslTcp` (both spellings accepted for compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    Tcp,
    Udp,
    SslTcp,
}

impl PortType {
    /// Iterate over every supported variant (used by tests).
    pub const ALL: [PortType; 3] = [PortType::Tcp, PortType::Udp, PortType::SslTcp];

    /// Convert to the canonical lowercase wire label.
    pub fn wire_label(self) -> &'static str {
        wire_label(self)
    }
}

/// Parse a wire-format `type` string into a [`PortType`].
///
/// Accepts (case-insensitive):
/// * `"tcp"` / `"TCP"`
/// * `"udp"` / `"UDP"`
/// * `"tcp_ssl"` / `"ssl-tcp"` (and their upper-case variants)
pub fn parse_port_type(s: &str) -> Result<PortType, AppError> {
    match s.to_ascii_lowercase().as_str() {
        "tcp" => Ok(PortType::Tcp),
        "udp" => Ok(PortType::Udp),
        "tcp_ssl" | "ssl-tcp" => Ok(PortType::SslTcp),
        other => Err(AppError::BadType(other.to_string())),
    }
}

/// Canonical lowercase wire label for a [`PortType`].
///
/// Used as the Prometheus `port_type` label so values stay stable across
/// releases.
pub fn wire_label(t: PortType) -> &'static str {
    match t {
        PortType::Tcp => "tcp",
        PortType::Udp => "udp",
        PortType::SslTcp => "ssl-tcp",
    }
}

/// Abstraction over a bound port that can accept client connections and
/// fan data in/out of the WebSocket dispatcher.
///
/// All methods must be safe to call from any thread (`Send + Sync`).
pub trait PortEntity: Send + Sync {
    /// The port number this entity is bound to.
    fn port(&self) -> PortNumber;

    /// Wire-level port type (TCP / UDP / SSL-TCP).
    fn kind(&self) -> PortType;

    /// Send `data` to a single connected client. Used by the WS server
    /// to push data from the API caller down to the remote peer.
    fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError>;

    /// Forcibly close a single client connection.
    fn close_client(&self, client: ClientId) -> Result<(), AppError>;

    /// Tear down the whole port: stop accepting new clients, close all
    /// existing connections, release the OS socket.
    fn shutdown(&self) -> Result<(), AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_port_type_accepts_all_legal_values() {
        assert_eq!(parse_port_type("tcp").unwrap(), PortType::Tcp);
        assert_eq!(parse_port_type("udp").unwrap(), PortType::Udp);
        assert_eq!(parse_port_type("tcp_ssl").unwrap(), PortType::SslTcp);
        assert_eq!(parse_port_type("ssl-tcp").unwrap(), PortType::SslTcp);
    }

    #[test]
    fn parse_port_type_is_case_insensitive() {
        assert_eq!(parse_port_type("TCP").unwrap(), PortType::Tcp);
        assert_eq!(parse_port_type("Udp").unwrap(), PortType::Udp);
        assert_eq!(parse_port_type("SSL-TCP").unwrap(), PortType::SslTcp);
        assert_eq!(parse_port_type("TCP_SSL").unwrap(), PortType::SslTcp);
    }

    #[test]
    fn parse_port_type_rejects_unknown_values() {
        for bad in ["", "http", "tls", "ssl", "tcp-ssl", "tcp_ssls"] {
            let err = parse_port_type(bad).expect_err("should reject");
            match err {
                AppError::BadType(s) => assert_eq!(s, bad.to_ascii_lowercase()),
                other => panic!("expected BadType, got {other:?}"),
            }
        }
    }

    #[test]
    fn wire_label_is_stable_and_lowercase() {
        assert_eq!(wire_label(PortType::Tcp), "tcp");
        assert_eq!(wire_label(PortType::Udp), "udp");
        assert_eq!(wire_label(PortType::SslTcp), "ssl-tcp");
        // The trait-method must agree with the free function.
        for t in PortType::ALL {
            assert_eq!(t.wire_label(), wire_label(t));
        }
    }

    #[test]
    fn wire_label_roundtrips_through_parse() {
        for t in PortType::ALL {
            let label = wire_label(t);
            let parsed = parse_port_type(label).expect("round-trip");
            assert_eq!(parsed, t, "round-trip mismatch for {label}");
        }
    }

    #[test]
    fn ws_event_debug_clone_eq() {
        let id = ClientId::from_u128(1);
        let e1 = WsEvent::Connected {
            client: id,
            addr: "127.0.0.1:1".into(),
        };
        let e2 = e1.clone();
        assert_eq!(e1, e2);

        let d1 = WsEvent::Data {
            client: id,
            data: "AA".into(),
            hex: true,
        };
        let _ = format!("{d1:?}");

        let c1 = WsEvent::Closed { client: id };
        assert_eq!(c1, WsEvent::Closed { client: id });

        let err = WsEvent::Error { msg: "boom".into() };
        let _ = format!("{err:?}");
    }
}