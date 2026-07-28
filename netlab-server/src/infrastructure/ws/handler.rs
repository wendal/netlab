//! WebSocket session lifecycle.
//!
//! A session runs three concurrent tasks:
//! 1. A *write* task that owns `ws_tx` and pumps outbound `Message`s
//!    from an internal `mpsc::UnboundedSender`.
//! 2. A *fan-in* task that drains the per-session `WsEvent` channel and
//!    re-publishes each event as a JSON text frame on the same internal
//!    channel — this keeps a single writer to the socket.
//! 3. The *read* loop (the current task) which decodes inbound text
//!    frames, hands them to [`ws_dispatch::handle`], and serialises the
//!    returned `Vec<serde_json::Value>` straight back to the client.
//!
//! On any of those loops terminating — `Close` frame, read error, or
//! write error — the session is closed: `close_session` releases every
//! port the session owned and the write task is aborted.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::application::ws_dispatch;
use crate::domain::port_entity::WsEvent;
use crate::infrastructure::http::router::AppState;

/// Drive a single WebSocket connection to completion.
pub async fn handle_ws_session(socket: WebSocket, state: Arc<AppState>, session_id: Uuid) {
    info!(%session_id, "ws session opened");

    let (mut ws_tx, mut ws_rx) = socket.split();

    // Single-writer channel: any source (events or read-loop replies)
    // publishes here, and the write task is the only owner of ws_tx.
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

    // Fan-in: forward WsEvent -> JSON text frame onto out_tx.
    if let Some(events_rx) = state.service.subscribe(session_id) {
        let out_tx_clone = out_tx.clone();
        tokio::spawn(async move {
            let mut events_rx = events_rx;
            while let Some(event) = events_rx.recv().await {
                let json = match serde_json::to_string(&event_to_json(&event)) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("serialize ws event: {e}");
                        continue;
                    }
                };
                if out_tx_clone.send(Message::Text(json)).is_err() {
                    // The write side has gone away; nothing to do.
                    break;
                }
            }
        });
    } else {
        warn!(%session_id, "ws session has no event channel");
    }

    // Write loop: out_rx -> ws_tx.
    let write_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Read loop.
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let outs = ws_dispatch::handle(&state.service, session_id, &text).await;
                for v in outs {
                    let s = serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string());
                    if out_tx.send(Message::Text(s)).is_err() {
                        // Writer is gone; bail out of the read loop too.
                        break;
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {
                // Binary / Ping / Pong frames are intentionally ignored
                // (axum handles Ping/Pong automatically at the codec level).
            }
            Err(e) => {
                warn!("ws read error: {e}");
                break;
            }
        }
    }

    // Tear down: release every port the session owned, then stop the
    // write task. We don't `await` close_session's completion strictly,
    // because we still want to log the session-closed event.
    state.service.close_session(session_id).await;
    write_task.abort();
    let _ = write_task.await;
    info!(%session_id, "ws session closed");
}

/// Project a domain [`WsEvent`] into the wire-format JSON object the
/// JavaScript front-end already speaks.
///
/// We serialise by hand (rather than via `serde::Serialize` on the enum)
/// so the field names stay decoupled from the Rust identifiers — a rename
/// on the Rust side is then a non-breaking change for clients.
fn event_to_json(e: &WsEvent) -> serde_json::Value {
    match e {
        WsEvent::Connected { client, addr } => serde_json::json!({
            "action": "connected",
            "client": client,
            "addr": addr,
        }),
        WsEvent::Data { client, data, hex } => serde_json::json!({
            "action": "data",
            "client": client,
            "data": data,
            "hex": hex,
        }),
        WsEvent::Closed { client } => serde_json::json!({
            "action": "closed",
            "client": client,
        }),
        WsEvent::Error { msg } => serde_json::json!({
            "action": "error",
            "msg": msg,
        }),
    }
}
