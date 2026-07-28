//! Process bootstrap: wire up logging, metrics, port service, and the
//! HTTP/WebSocket server, then run until SIGINT.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::application::port_pool::{RandomPortPool, SharedPortPool};
use crate::application::port_service::{EntityFactory, PortService};
use crate::config::NetlabConfig;
use crate::domain::errors::AppError;
use crate::domain::port::PortNumber;
use crate::domain::port_entity::{PortEntity, PortType, WsEvent};
use crate::infrastructure::http::router::{build_router, AppState};
use crate::infrastructure::metrics::exporter;
use crate::infrastructure::tcp::entity::TcpPortEntity;
use crate::infrastructure::tcp::tls::TlsMaterial;
use crate::infrastructure::udp::entity::UdpPortEntity;

/// Concrete factory: TCP/UDP/TLS-TCP bound ports via tokio.
pub struct DefaultEntityFactory {
    tls: Option<Arc<TlsMaterial>>,
}

impl DefaultEntityFactory {
    pub fn new(tls: Option<Arc<TlsMaterial>>) -> Self {
        Self { tls }
    }
}

#[async_trait]
impl EntityFactory for DefaultEntityFactory {
    async fn create(
        &self,
        port: PortNumber,
        kind: PortType,
        events: mpsc::UnboundedSender<WsEvent>,
    ) -> Result<Arc<dyn PortEntity>, AppError> {
        let entity: Arc<dyn PortEntity> = match kind {
            PortType::Tcp => TcpPortEntity::start(port, false, events, None)
                .await?
                as Arc<dyn PortEntity>,
            PortType::SslTcp => {
                TcpPortEntity::start(port, true, events, self.tls.clone())
                    .await?
                    as Arc<dyn PortEntity>
            }
            PortType::Udp => UdpPortEntity::start(port, events).await? as Arc<dyn PortEntity>,
        };
        Ok(entity)
    }
}

#[derive(Clone)]
pub struct AppContext {
    pub config: NetlabConfig,
    pub service: Arc<PortService>,
}

impl AppContext {
    pub fn new(config: NetlabConfig, service: Arc<PortService>) -> Self {
        Self { config, service }
    }
}

/// Initialize tracing subscriber. Idempotent.
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();
}

pub async fn run() -> anyhow::Result<()> {
    init_tracing();

    let cfg = NetlabConfig::load().context("loading config")?;
    info!(
        "netlab-server starting; http={}:{}, prom=:{}, port range={}-{}",
        cfg.server.host,
        cfg.server.port,
        cfg.metrics.port,
        cfg.port.start,
        cfg.port.end
    );

    // Install Prometheus recorder. Returns the handle for the axum route.
    exporter::install().context("installing prometheus recorder")?;

    // Optional TLS material for the SslTcp variant. Failures fall back to
    // a self-signed cert (see TlsMaterial::from_pem_files).
    let tls = match (&cfg.ssl.cert_path, &cfg.ssl.key_path) {
        (Some(cert), Some(key)) => TlsMaterial::from_pem_files(
            cert,
            key,
            cfg.ssl.key_password.as_deref(),
        )
        .or_else(|e| {
            warn!(
                "loading PEM TLS from {cert}/{key} failed: {e}; \
                 using self-signed cert (dev only)"
            );
            TlsMaterial::self_signed()
        })
        .ok(),
        _ => {
            warn!("ssl.cert_path / ssl.key_path not set; using self-signed cert (dev only)");
            TlsMaterial::self_signed().ok()
        }
    };

    // Build the port pool + service.
    let range = crate::domain::port::PortRange::new(cfg.port.start, cfg.port.end)
        .context("invalid port range")?;
    let pool: SharedPortPool = Arc::new(RandomPortPool::new(range));
    let factory = Arc::new(DefaultEntityFactory::new(tls));
    let service = PortService::new(pool, factory);

    // Build the axum router and the AppState.
    let state = Arc::new(AppState {
        service: service.clone(),
    });
    let router = build_router(state, &cfg.app.static_dir);

    // Bind the main HTTP+WS listener.
    let http_addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port)
        .parse()
        .context("invalid server address")?;
    let listener = tokio::net::TcpListener::bind(http_addr)
        .await
        .with_context(|| format!("binding {http_addr}"))?;
    info!("http+ws listening on {http_addr}");

    // Spawn the independent Prometheus listener on :9400 (per plan).
    if cfg.metrics.enabled {
        exporter::spawn_prometheus_listener(cfg.metrics.port)
            .context("spawning prometheus listener")?;
        info!("prometheus listening on :{}", cfg.metrics.port);
    }

    // Run the server. Ctrl-C triggers a graceful shutdown.
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum::serve")?;

    info!("server stopped; closing any open ports");
    // PortService has no explicit close-all; entities drop on Arc refcount
    // going to zero when the service is dropped.
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
