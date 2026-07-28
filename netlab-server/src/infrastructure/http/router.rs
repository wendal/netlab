//! Top-level axum router assembly.
//!
//! The router composes three concerns:
//! 1. `GET /metrics` — Prometheus scrape endpoint.
//! 2. `GET /ws/netlab` — WebSocket upgrade entry point.
//! 3. Everything else — served by the static-file sub-router.
//!
//! The shared [`AppState`] is wrapped in an [`Arc`] and threaded through
//! axum's `with_state`. Each sub-router is merged in (rather than nested
//! as a fallback) so that `WebSocketUpgrade` extraction and `IntoResponse`
//! handling stay independent of the static-file service.

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::application::port_service::PortService;
use crate::infrastructure::http::static_files::static_router;
use crate::infrastructure::metrics::exporter::metrics_handler;
use crate::infrastructure::ws::endpoint::ws_router;

/// Shared axum state handed to every handler that needs it.
pub struct AppState {
    /// Application-layer port service that owns the port pool and the
    /// `WsEvent` dispatch channels.
    pub service: Arc<PortService>,
    // Future fields: metrics handle, config, etc.
}

/// Build the full HTTP+WS router.
///
/// `state` is shared by every handler that needs it. `static_dir` is the
/// on-disk path to the built web UI; the static sub-router is merged in
/// so that any unmatched request falls through to it.
///
/// The stateful WebSocket sub-router is the only consumer of `state`,
/// so we apply it locally with `with_state` — that collapses
/// `Router<Arc<AppState>>` down to `Router<()>`, which lines up with
/// the stateless metrics and static sub-routers and keeps the outer
/// `Router` type simple.
pub fn build_router(state: Arc<AppState>, static_dir: &str) -> Router {
    let ws = ws_router().with_state(state);
    Router::new()
        .route("/metrics", get(metrics_handler))
        .merge(ws)
        .merge(static_router(static_dir))
}
