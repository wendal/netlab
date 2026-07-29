//! TCP / SSL-TCP port entity.
//!
//! `TcpPortEntity` owns a bound [`TcpListener`] and an accept loop. Every
//! incoming connection is dispatched to a dedicated task that:
//!
//! 1. (Optionally) wraps the socket in `tokio_rustls` for TLS.
//! 2. Runs a single `tokio::select!` loop that multiplexes:
//!    - network reads (via a [`TcpProtocol`] strategy, `DumpProtocol` by
//!      default), emitting [`WsEvent::Data`] for each chunk.
//!    - outbound writes fed by an `mpsc::UnboundedReceiver`, which is
//!      what [`PortEntity::send`] produces.
//! 3. Registers the client in [`entity.clients`] so the application layer
//!      can find it while it's connected.
//! 4. Emits [`WsEvent::Closed`] on EOF and removes the client from the
//!      registry so the connection cleanly tears down.
//!
//! The shape mirrors the Java `TcpPortEntity` from the original
//! `luatos-netlab` server, ported to `tokio`.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::domain::client::{new_client_id, ClientEndpoint, ClientId, ClientStat};
use crate::domain::errors::AppError;
use crate::domain::hex;
use crate::domain::port_entity::{wire_label, PortEntity, PortType, WsEvent};
use crate::infrastructure::tcp::protocol::{DumpProtocol, TcpProtocol};
use crate::infrastructure::tcp::tls::TlsMaterial;

/// Combined read + write trait so we can build a single trait object for
/// either a plain TCP stream or a TLS-wrapped one.
trait AsyncReadWrite: AsyncRead + AsyncWrite + Send {}
impl<T: AsyncRead + AsyncWrite + Send + ?Sized> AsyncReadWrite for T {}

/// Per-connection state held inside [`TcpPortEntity::clients`].
#[derive(Debug)]
struct ClientSlot {
    /// Channel into the connection task. Dropping the sender is how
    /// `close_client` / `shutdown` signal the writer to exit.
    writer: mpsc::UnboundedSender<Vec<u8>>,
    /// Per-connection counters shared with the read loop.
    stat: Arc<ClientStat>,
}

/// A bound TCP port, with or without TLS.
#[derive(Debug)]
pub struct TcpPortEntity {
    port: u16,
    kind: PortType,
    clients: Mutex<HashMap<ClientId, ClientSlot>>,
    accept_task: Mutex<Option<JoinHandle<()>>>,
    shutdown_flag: Arc<AtomicBool>,
}

impl TcpPortEntity {
    /// Bind a TCP socket on `port` and start the accept loop.
    ///
    /// Returns immediately; the accept loop runs on a background task.
    pub async fn start(
        port: u16,
        use_tls: bool,
        events: mpsc::UnboundedSender<WsEvent>,
        tls: Option<Arc<TlsMaterial>>,
    ) -> Result<Arc<Self>, AppError> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(|e| AppError::PortBind(format!("tcp bind {port}: {e}")))?;
        let bound_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);

        let entity = Arc::new(Self {
            port: bound_port,
            kind: if use_tls {
                PortType::SslTcp
            } else {
                PortType::Tcp
            },
            clients: Mutex::new(HashMap::new()),
            accept_task: Mutex::new(None),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        });

        let kind = entity.kind;
        let events_tx = events.clone();
        let entity_weak = Arc::downgrade(&entity);
        let shutdown_flag = entity.shutdown_flag.clone();
        let tls = tls.clone();
        let proto: Arc<dyn TcpProtocol> = Arc::new(DumpProtocol);

        let handle = tokio::spawn(async move {
            info!("tcp: accept loop started on port {bound_port} (kind={kind:?})");
            loop {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                let (stream, peer) = match listener.accept().await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("tcp accept error: {e}");
                        tokio::task::yield_now().await;
                        continue;
                    }
                };

                let client_id = new_client_id();
                let addr_str = ClientEndpoint::new(peer.ip().to_string(), peer.port()).to_string();

                let events_c = events_tx.clone();
                let proto_c = proto.clone();
                let tls_c = tls.clone();
                let entity_weak_c = entity_weak.clone();
                let kind_c = kind;
                let addr_label = addr_str.clone();

                tokio::spawn(async move {
                    // The TLS handshake is performed inside the per-connection
                    // task so a slow / failing client does not block the
                    // accept loop.
                    let res = if let Some(tls) = tls_c.as_ref() {
                        match tls.acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                drive_connection(
                                    Box::pin(tls_stream),
                                    client_id,
                                    addr_str,
                                    kind_c,
                                    events_c,
                                    proto_c,
                                    entity_weak_c,
                                )
                                .await
                            }
                            Err(e) => {
                                warn!("tcp tls handshake failed from {addr_label}: {e}");
                                let _ = events_c.send(WsEvent::Error {
                                    msg: format!("tls handshake: {e}"),
                                });
                                Ok(())
                            }
                        }
                    } else {
                        drive_connection(
                            Box::pin(stream),
                            client_id,
                            addr_str,
                            kind_c,
                            events_c,
                            proto_c,
                            entity_weak_c,
                        )
                        .await
                    };

                    if let Err(e) = res {
                        error!("tcp conn {addr_label} error: {e}");
                    }
                });
            }
            info!("tcp: accept loop stopped on port {bound_port}");
        });

        *entity.accept_task.lock() = Some(handle);
        Ok(entity)
    }
}

/// Drive one TCP connection: read / write loop, event emission, cleanup.
///
/// Takes the stream as a type-erased `Pin<Box<dyn AsyncRead + AsyncWrite>>`
/// so the same body serves both the plain TCP and the TLS-wrapped path.
async fn drive_connection(
    mut stream: Pin<Box<dyn AsyncReadWrite>>,
    client_id: ClientId,
    addr_str: String,
    kind: PortType,
    events: mpsc::UnboundedSender<WsEvent>,
    _proto: Arc<dyn TcpProtocol>,
    entity_weak: std::sync::Weak<TcpPortEntity>,
) -> Result<(), AppError> {
    let _ = events.send(WsEvent::Connected {
        client: client_id,
        addr: addr_str.clone(),
    });
    metrics::gauge!("netlab_clients_open", "port_type" => wire_label(kind)).increment(1.0);

    // Register the client BEFORE we start reading so that any inbound
    // `send` from the application layer can find it.
    let stat = Arc::new(ClientStat::new());
    let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    if let Some(entity) = entity_weak.upgrade() {
        entity.clients.lock().insert(
            client_id,
            ClientSlot {
                writer: writer_tx.clone(),
                stat: stat.clone(),
            },
        );
    }

    let mut read_buf = vec![0u8; 16 * 1024];

    // Single-task loop: read chunks OR pull data from the writer channel.
    let exit_reason = loop {
        tokio::select! {
            // Outbound data from the application layer.
            maybe_data = writer_rx.recv() => {
                match maybe_data {
                    Some(data) => {
                        if let Err(e) = stream.write_all(&data).await {
                            warn!("tcp write failed for {addr_str}: {e}");
                            break "write_error";
                        }
                        if let Err(e) = stream.flush().await {
                            warn!("tcp flush failed for {addr_str}: {e}");
                            break "flush_error";
                        }
                    }
                    None => {
                        // Sender dropped -> entity shutdown or close_client.
                        break "sender_dropped";
                    }
                }
            }
            // Inbound data from the peer.
            read_res = stream.read(&mut read_buf) => {
                match read_res {
                    Ok(0) => {
                        debug!("tcp: peer {addr_str} closed");
                        break "peer_closed";
                    }
                    Ok(n) => {
                        let bytes = &read_buf[..n];
                        let hex_str = hex::encode_bytes_to_hex(bytes);
                        let _ = events.send(WsEvent::Data {
                            client: client_id,
                            data: hex_str,
                            hex: true,
                        });
                        stat.add_rx(n as u64);
                        metrics::counter!(
                            "netlab_bytes_total",
                            "port_type" => wire_label(kind),
                            "dir" => "rx"
                        )
                        .increment(n as u64);
                    }
                    Err(e) => {
                        let _ = events.send(WsEvent::Error {
                            msg: format!("{addr_str}: {e}"),
                        });
                        break "read_error";
                    }
                }
            }
        }
    };

    // Drop the writer sender so the entity's bookkeeping reflects the
    // closed connection.
    drop(writer_tx);

    if let Some(entity) = entity_weak.upgrade() {
        entity.clients.lock().remove(&client_id);
    }
    let _ = events.send(WsEvent::Closed { client: client_id });
    metrics::gauge!("netlab_clients_open", "port_type" => wire_label(kind)).decrement(1.0);

    debug!("tcp: connection {addr_str} ended ({exit_reason})");
    Ok(())
}

#[async_trait]
impl PortEntity for TcpPortEntity {
    fn port(&self) -> u16 {
        self.port
    }

    fn kind(&self) -> PortType {
        self.kind
    }

    fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError> {
        let (writer, stat) = {
            let clients = self.clients.lock();
            let slot = clients
                .get(&client)
                .ok_or_else(|| AppError::UnknownClient(client.to_string()))?;
            (slot.writer.clone(), slot.stat.clone())
        };
        writer
            .send(data.to_vec())
            .map_err(|_| AppError::UnknownClient(client.to_string()))?;
        stat.add_tx(data.len() as u64);
        metrics::counter!(
            "netlab_bytes_total",
            "port_type" => wire_label(self.kind),
            "dir" => "tx"
        )
        .increment(data.len() as u64);
        Ok(())
    }

    fn close_client(&self, client: ClientId) -> Result<(), AppError> {
        // Removing the slot drops the `UnboundedSender`, which causes the
        // connection task to observe `recv()` returning `None` and exit.
        let mut clients = self.clients.lock();
        if clients.remove(&client).is_none() {
            return Err(AppError::UnknownClient(client.to_string()));
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<(), AppError> {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        // Drop every per-client sender. The connection tasks will exit
        // their write arms and tear down their streams.
        self.clients.lock().clear();
        if let Some(h) = self.accept_task.lock().take() {
            h.abort();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // The entity is exercised through the integration test in
    // `tests/tcp_port.rs`. Unit tests here would need a real listener which
    // is exactly what the integration test already does, so we skip them.
}
