//! WebSocket HTTP endpoint.
//!
//! Exposes `GET /ws/netlab`. On a successful upgrade a new [`Uuid`] is
//! minted as the *session id* and the connection is handed to
//! [`handle_ws_session`]. The session id scopes every downstream
//! `WsEvent` and `new_port` call so a single client may own many ports.

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::get;
use uuid::Uuid;

use crate::infrastructure::http::router::AppState;
use crate::infrastructure::ws::handler::handle_ws_session;

/// Build the WebSocket sub-router.
///
/// The returned router is generic over `Arc<AppState>`; callers wrap it
/// with the shared state via `with_state` when assembling the top-level
/// router.
pub fn ws_router() -> Router<Arc<AppState>> {
    Router::new().route("/ws/netlab", get(ws_upgrade))
}

/// HTTP→WS upgrade handler.
///
/// Each upgrade receives a fresh session id. The id is used by the
/// application layer to scope subscriptions, so concurrent WS sessions
/// never see each other's events.
async fn ws_upgrade(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let session_id = Uuid::new_v4();
    ws.on_upgrade(move |socket| handle_ws_session(socket, state, session_id))
}
