//! UDP "client" context: one remote peer plus its per-client stats.
//!
//! UDP is connectionless, so the analogue of a "client" is the remote
//! `SocketAddr` of a peer that has sent at least one datagram to us.
//! This mirrors `UdpClient` in Java's `UdpPortEntity.java`.

use std::net::SocketAddr;
use std::sync::Arc;

use crate::domain::client::{ClientId, ClientStat};

/// Per-peer context tracked by [`UdpPortEntity`](super::entity::UdpPortEntity).
///
/// `stat` is wrapped in `Arc` so the read loop can update counters in place
/// while a `send()` task may independently reference the same counters.
pub struct UdpClientCtx {
    pub id: ClientId,
    pub addr: SocketAddr,
    pub stat: Arc<ClientStat>,
}

impl UdpClientCtx {
    /// Build a fresh context for a peer we have never seen before.
    pub fn new(id: ClientId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            stat: Arc::new(ClientStat::new()),
        }
    }
}