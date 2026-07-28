//! Prometheus metrics exporter.
//!
//! Two integration modes are supported, in order of preference:
//!
//! 1. **Inline (default).** [`install`] registers the global Prometheus
//!    recorder and stashes the handle in a process-wide [`Lazy`] static.
//!    [`metrics_handler`] then renders the scrape body for the
//!    `GET /metrics` route on the main HTTP port (`:9073`).
//! 2. **Sidecar (opt-in).** [`spawn_prometheus_listener`] boots a tiny
//!    axum server on its own port (typically `:9400`) that only serves
//!    `/metrics`. This matches the Java reference implementation.
//!
//! Both modes are independent — the inline handler always works once
//! [`install`] has been called, even if no sidecar is running.

use std::net::SocketAddr;

use axum::Router;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Process-wide Prometheus handle. `None` until [`install`] succeeds.
static HANDLE: Lazy<Mutex<Option<metrics_exporter_prometheus::PrometheusHandle>>> =
    Lazy::new(|| Mutex::new(None));

/// Install the Prometheus recorder.
///
/// This only sets up the global recorder; the metrics are exposed via
/// the inline [`metrics_handler`] on the main HTTP port. Safe to call
/// once at process start.
pub fn install() -> anyhow::Result<()> {
    let handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()?;
    *HANDLE.lock() = Some(handle);
    Ok(())
}

/// Install the recorder and, if requested, also spawn a sidecar
/// `:port` listener that only serves `/metrics`.
///
/// The inline handler keeps working on the main port regardless of
/// whether the sidecar is started.
pub fn install_with_port(port: u16) -> anyhow::Result<()> {
    install()?;
    spawn_prometheus_listener(port)?;
    Ok(())
}

/// Spawn an axum-based `/metrics`-only server on `0.0.0.0:port`.
///
/// Returns immediately; the server runs in a detached tokio task. The
/// function only fails when binding the listener fails — once the
/// server is up, runtime errors are logged inside the task.
pub fn spawn_prometheus_listener(port: u16) -> anyhow::Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let app = Router::new().route("/metrics", get(metrics_handler));
    tokio::spawn(async move {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                if let Err(e) = axum::serve(listener, app).await {
                    tracing::error!("prometheus listener exited: {e}");
                }
            }
            Err(e) => tracing::error!("failed to bind prometheus listener on {addr}: {e}"),
        }
    });
    Ok(())
}

/// `GET /metrics` handler.
///
/// Renders the current snapshot of all registered counters/gauges/
/// histograms in the Prometheus text exposition format. If the recorder
/// has not been installed yet (misconfigured boot), a single explanatory
/// comment line is returned with a `text/plain` content type.
pub async fn metrics_handler() -> impl IntoResponse {
    let guard = HANDLE.lock();
    match guard.as_ref() {
        Some(handle) => {
            let body = handle.render();
            (
                [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
                body,
            )
        }
        None => (
            [(header::CONTENT_TYPE, "text/plain")],
            "# metrics not installed\n".to_string(),
        ),
    }
}
