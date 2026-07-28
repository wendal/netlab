//! Single-entity, single-thread UDP port implementation.
//!
//! One [`UdpPortEntity`] owns a single bound [`tokio::net::UdpSocket`] and
//! spawns one background task that drains incoming datagrams. Peers are
//! identified by their `SocketAddr`: the first datagram from a given peer
//! emits a [`WsEvent::Connected`], subsequent datagrams only emit
//! [`WsEvent::Data`]. UDP never emits [`WsEvent::Closed`] — there is no
//! connection to close, and `close_client` only drops the bookkeeping
//! entries.
//!
//! This matches Java's `UdpPortEntity.java` (which keeps no closed-event
//! path for UDP).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::application::metrics;
use crate::domain::client::{new_client_id, ClientId};
use crate::domain::errors::AppError;
use crate::domain::hex::encode_bytes_to_hex;
use crate::domain::port_entity::{wire_label, PortEntity, PortType, WsEvent};
use crate::infrastructure::udp::client::UdpClientCtx;

/// Bound UDP port plus its client bookkeeping and read-loop handle.
pub struct UdpPortEntity {
    port: u16,
    socket: Arc<UdpSocket>,
    clients: Mutex<HashMap<SocketAddr, UdpClientCtx>>,
    id_to_addr: Mutex<HashMap<ClientId, SocketAddr>>,
    read_task: Mutex<Option<JoinHandle<()>>>,
    shutdown_flag: Arc<AtomicBool>,
}

impl UdpPortEntity {
    /// Bind a UDP socket on `port` and spawn the background read loop.
    ///
    /// `port == 0` requests an ephemeral port; the actual bound port is
    /// available via [`PortEntity::port`] on the returned entity.
    pub async fn start(
        port: u16,
        events: mpsc::UnboundedSender<WsEvent>,
    ) -> Result<Arc<Self>, AppError> {
        // Bind to the IPv4 wildcard so dual-stack quirks don't bite us on
        // every platform (Windows in particular is finicky about `::`
        // accepting IPv4 peers). The Java reference uses an IPv4 socket.
        let socket = UdpSocket::bind(("0.0.0.0", port))
            .await
            .map_err(|e| AppError::PortBind(format!("udp bind {port}: {e}")))?;
        let bound_port = socket.local_addr().map(|a| a.port()).unwrap_or(port);

        let entity = Arc::new(Self {
            port: bound_port,
            socket: Arc::new(socket),
            clients: Mutex::new(HashMap::new()),
            id_to_addr: Mutex::new(HashMap::new()),
            read_task: Mutex::new(None),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        });

        let socket = entity.socket.clone();
        let entity_weak = Arc::downgrade(&entity);
        let events = events;
        let shutdown = entity.shutdown_flag.clone();

        let handle = tokio::spawn(async move {
            // 65535 is the theoretical max UDP datagram payload; tokio
            // returns the truncated size from recv_from if a larger packet
            // arrives, so this buffer never overflows.
            let mut buf = vec![0u8; 65535];
            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                let (n, peer) = match socket.recv_from(&mut buf).await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("udp recv error: {e}");
                        continue;
                    }
                };
                let entity = match entity_weak.upgrade() {
                    Some(e) => e,
                    None => break, // entity dropped -> exit
                };

                let (client_id, is_new) = {
                    let mut clients = entity.clients.lock();
                    if let Some(ctx) = clients.get_mut(&peer) {
                        ctx.stat.add_rx(n as u64);
                        (ctx.id, false)
                    } else {
                        let id = new_client_id();
                        let ctx = UdpClientCtx::new(id, peer);
                        clients.insert(peer, ctx);
                        let mut id_map = entity.id_to_addr.lock();
                        id_map.insert(id, peer);
                        (id, true)
                    }
                };

                if is_new {
                    let _ = events.send(WsEvent::Connected {
                        client: client_id,
                        addr: peer.to_string(),
                    });
                    metrics::on_client_open(wire_label(PortType::Udp));
                }

                let data_hex = encode_bytes_to_hex(&buf[..n]);
                let _ = events.send(WsEvent::Data {
                    client: client_id,
                    data: data_hex,
                    hex: true,
                });
                metrics::on_data(wire_label(PortType::Udp), n as u64);
            }
        });

        *entity.read_task.lock() = Some(handle);
        Ok(entity)
    }
}

#[async_trait::async_trait]
impl PortEntity for UdpPortEntity {
    fn port(&self) -> u16 {
        self.port
    }

    fn kind(&self) -> PortType {
        PortType::Udp
    }

    /// Send `data` to the peer identified by `client`.
    ///
    /// UDP send is fire-and-forget from the caller's perspective: we
    /// spawn the actual `send_to` so the caller doesn't block on the
    /// kernel. Stats are only updated if the spawned send succeeds.
    fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError> {
        let addr = *self
            .id_to_addr
            .lock()
            .get(&client)
            .ok_or_else(|| AppError::UnknownClient(client.to_string()))?;
        let socket = self.socket.clone();
        let data = data.to_vec();

        let stat = {
            let clients = self.clients.lock();
            clients.get(&addr).map(|c| c.stat.clone())
        };

        tokio::spawn(async move {
            if let Err(e) = socket.send_to(&data, addr).await {
                warn!("udp send to {addr} error: {e}");
            } else if let Some(stat) = stat {
                stat.add_tx(data.len() as u64);
            }
        });
        Ok(())
    }

    /// Drop the bookkeeping entries for `client`.
    ///
    /// UDP never emits a `WsEvent::Closed` (matches Java `UdpPortEntity.java`
    /// which has no closed-event path for UDP).
    fn close_client(&self, client: ClientId) -> Result<(), AppError> {
        let addr = self
            .id_to_addr
            .lock()
            .remove(&client)
            .ok_or_else(|| AppError::UnknownClient(client.to_string()))?;
        self.clients.lock().remove(&addr);
        Ok(())
    }

    /// Stop the read loop and release the socket.
    fn shutdown(&self) -> Result<(), AppError> {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.read_task.lock().take() {
            h.abort();
        }
        Ok(())
    }
}