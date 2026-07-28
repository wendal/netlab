//! luatos-netlab Rust backend
//!
//! Three-layer Clean Architecture:
//! - `domain`      : pure types, no IO
//! - `application` : use cases (port pool, port service, ws dispatch, metrics)
//! - `infrastructure` : axum http/ws, tokio tcp/udp, tls, prometheus exporter

pub mod application;
pub mod bootstrap;
pub mod config;
pub mod domain;
pub mod infrastructure;

pub use bootstrap::AppContext;
pub use config::NetlabConfig;
