//! Static-file serving for the web UI.
//!
//! All `GET /` traffic that does not match a more specific route falls
//! through to a [`ServeDir`] rooted at the configured `static_dir`. When
//! the directory is missing (typical for a clean checkout) we degrade
//! gracefully to a `404` instead of panicking during boot.

use std::path::Path;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

/// Build the static-file sub-router.
///
/// Behaviour:
/// * When `static_dir` points at an existing directory, serve it via
///   `tower_http::services::ServeDir` with `append_index_html_on_directories`
///   enabled so `/` renders `index.html`.
/// * When the directory is absent, every request gets a `404 Not Found`
///   with a short human-readable body — the rest of the API (WebSocket
///   and `/metrics`) keeps working without the web UI.
pub fn static_router(static_dir: &str) -> Router {
    let dir = Path::new(static_dir);
    if dir.is_dir() {
        Router::new().fallback_service(ServeDir::new(dir).append_index_html_on_directories(true))
    } else {
        Router::new().fallback(get(missing_static_dir))
    }
}

async fn missing_static_dir() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "static dir not found")
}
