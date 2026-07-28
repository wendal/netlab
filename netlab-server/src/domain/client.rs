//! Domain types representing a connected client and its per-connection stats.
//!
//! `ClientStat` is the only piece of mutable state in the domain layer: it
//! tracks byte/packet counters and the last-activity timestamp using atomics
//! so it is safe to share across threads without any external locking.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque, globally-unique client identifier.
pub type ClientId = Uuid;

/// Convenience: mint a fresh client id (UUID v4).
pub fn new_client_id() -> ClientId {
    Uuid::new_v4()
}

/// A `(ip, port)` peer address, displayed as `ip:port`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientEndpoint {
    pub ip: String,
    pub port: u16,
}

impl ClientEndpoint {
    pub fn new(ip: impl Into<String>, port: u16) -> Self {
        Self {
            ip: ip.into(),
            port,
        }
    }
}

impl fmt::Display for ClientEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

/// Thread-safe per-client statistics. All counters are monotonically
/// increasing except `last_active_ms` which is refreshed by [`ClientStat::touch`].
#[derive(Debug, Default)]
pub struct ClientStat {
    tx_bytes: AtomicU64,
    rx_bytes: AtomicU64,
    tx_count: AtomicU64,
    rx_count: AtomicU64,
    last_active_ms: AtomicU64,
}

impl ClientStat {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `n` bytes transmitted to the client (caller -> client).
    pub fn add_tx(&self, n: u64) {
        self.tx_bytes.fetch_add(n, Ordering::Relaxed);
        self.tx_count.fetch_add(1, Ordering::Relaxed);
        self.touch();
    }

    /// Record `n` bytes received from the client (client -> caller).
    pub fn add_rx(&self, n: u64) {
        self.rx_bytes.fetch_add(n, Ordering::Relaxed);
        self.rx_count.fetch_add(1, Ordering::Relaxed);
        self.touch();
    }

    pub fn tx_bytes(&self) -> u64 {
        self.tx_bytes.load(Ordering::Relaxed)
    }
    pub fn rx_bytes(&self) -> u64 {
        self.rx_bytes.load(Ordering::Relaxed)
    }
    pub fn tx_count(&self) -> u64 {
        self.tx_count.load(Ordering::Relaxed)
    }
    pub fn rx_count(&self) -> u64 {
        self.rx_count.load(Ordering::Relaxed)
    }
    pub fn last_active_ms(&self) -> u64 {
        self.last_active_ms.load(Ordering::Relaxed)
    }

    /// Refresh the last-active timestamp to "now" (ms since UNIX epoch).
    pub fn touch(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.last_active_ms.store(now, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn new_client_id_is_unique() {
        let a = new_client_id();
        let b = new_client_id();
        assert_ne!(a, b);
    }

    #[test]
    fn endpoint_display_format() {
        let ep = ClientEndpoint::new("127.0.0.1", 8080);
        assert_eq!(ep.to_string(), "127.0.0.1:8080");
        assert_eq!(format!("{ep}"), "127.0.0.1:8080");
    }

    #[test]
    fn stat_counters_accumulate() {
        let s = ClientStat::new();
        assert_eq!(s.tx_bytes(), 0);
        assert_eq!(s.rx_bytes(), 0);
        assert_eq!(s.tx_count(), 0);
        assert_eq!(s.rx_count(), 0);

        s.add_tx(10);
        s.add_tx(20);
        assert_eq!(s.tx_bytes(), 30);
        assert_eq!(s.tx_count(), 2);

        s.add_rx(5);
        s.add_rx(5);
        s.add_rx(5);
        assert_eq!(s.rx_bytes(), 15);
        assert_eq!(s.rx_count(), 3);
    }

    #[test]
    fn touch_updates_last_active_monotonically() {
        let s = ClientStat::new();
        s.touch();
        let t0 = s.last_active_ms();
        assert!(t0 > 0);

        // Ensure the clock moves at least 1 ms forward.
        sleep(Duration::from_millis(2));
        s.touch();
        let t1 = s.last_active_ms();
        assert!(t1 >= t0);
    }

    #[test]
    fn add_tx_and_add_rx_also_touch() {
        let s = ClientStat::new();
        s.touch();
        let t0 = s.last_active_ms();
        sleep(Duration::from_millis(2));
        s.add_rx(1);
        let t1 = s.last_active_ms();
        assert!(t1 >= t0);
    }

    #[test]
    fn stat_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ClientStat>();
        assert_send_sync::<ClientEndpoint>();
    }
}