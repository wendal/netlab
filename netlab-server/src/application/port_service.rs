//! Application-level port orchestration.
//!
//! `PortService` is the single entry point used by the WebSocket
//! dispatcher. It owns:
//!
//! * the [`PortPool`] (a free/busy tracker for port numbers),
//! * an [`EntityFactory`] that materialises concrete [`PortEntity`]
//!   implementations for the requested transport,
//! * per-session `WsEvent` channels that the WS handler subscribes to
//!   via [`PortService::subscribe`],
//! * bidirectional `session_id ⇄ entity` indices so `close_session`
//!   can shut everything down in one call.
//!
//! All state is in-memory and behind parking_lot mutexes — the
//! service is intended to live in an `Arc` and be shared between the
//! HTTP/WS layer and any background task that needs to emit metrics.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::application::metrics;
use crate::application::port_pool::SharedPortPool;
use crate::domain::client::ClientId;
use crate::domain::errors::AppError;
use crate::domain::port::PortNumber;
use crate::domain::port_entity::{parse_port_type, wire_label, PortEntity, PortType, WsEvent};

/// Token required when the caller asks for a specific port.
///
/// Any port that the WS caller pins to a number is privileged — we
/// only honour the request if the caller knows this shared secret.
/// Calls that don't pin a port are unprivileged and never need a
/// token.
pub const PORT_PIN_TOKEN: &str = "LuatOS-NetLab";

/// Factory used by [`PortService`] to materialise a concrete
/// [`PortEntity`] for a given (port, kind) pair.
///
/// Infrastructure code (TCP / UDP / SSL-TCP) implements this trait
/// and injects the factory at startup. The factory is responsible for
/// binding the socket, spawning accept loops, etc. — [`PortService`]
/// only orchestrates the higher-level lifecycle.
#[async_trait]
pub trait EntityFactory: Send + Sync {
    /// Build a new [`PortEntity`] bound to `port`, emitting its
    /// events through `events`. The returned `Arc` is stored by
    /// `PortService` for the lifetime of the session.
    async fn create(
        &self,
        port: PortNumber,
        kind: PortType,
        events: mpsc::UnboundedSender<WsEvent>,
    ) -> Result<Arc<dyn PortEntity>, AppError>;
}

/// Operations on the port service that the WebSocket dispatcher
/// needs. Defined as a trait (instead of being hard-coded to
/// `PortService`) so that `ws_dispatch::handle` can be unit-tested
/// with a mock.
#[async_trait]
pub trait WsPortOps: Send + Sync {
    /// Allocate a new port (see [`PortService::new_port`]).
    async fn new_port(
        &self,
        session: Uuid,
        type_str: &str,
        port: Option<u16>,
        token: Option<&str>,
    ) -> Result<u16, AppError>;

    /// Push `data` to a previously connected `client`.
    async fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError>;

    /// Forcibly close `client`.
    async fn close_client(&self, client: ClientId) -> Result<(), AppError>;

    /// Update the per-session broadcast flag. Currently a stub; the
    /// hook is here so the WS layer can already wire the message.
    fn set_broadcast(&self, session: Uuid, b: bool);
}

struct SessionEntry {
    port: PortNumber,
    kind: PortType,
    entity: Arc<dyn PortEntity>,
}

/// The application-layer port orchestrator.
///
/// Returned from [`PortService::new`] as `Arc<Self>` so the WS handler
/// can hold a clonable handle.
pub struct PortService {
    pool: SharedPortPool,
    factory: Arc<dyn EntityFactory>,
    /// session_id -> entities created in that session.
    sessions: Mutex<HashMap<Uuid, Vec<SessionEntry>>>,
    /// port -> session_id (reverse index, used during `close_session`).
    port_to_session: Mutex<HashMap<PortNumber, Uuid>>,
    /// session_id -> sender half of the session's `WsEvent` channel.
    ///
    /// The receiver is handed to the WS layer via [`subscribe`].
    /// Channels are created lazily (on first `new_port` or
    /// `subscribe`) so a session that never produces events doesn't
    /// allocate one.
    event_senders: Mutex<HashMap<Uuid, mpsc::UnboundedSender<WsEvent>>>,
}

impl PortService {
    /// Build a new service. Usually called once at startup.
    pub fn new(pool: SharedPortPool, factory: Arc<dyn EntityFactory>) -> Arc<Self> {
        Arc::new(Self {
            pool,
            factory,
            sessions: Mutex::new(HashMap::new()),
            port_to_session: Mutex::new(HashMap::new()),
            event_senders: Mutex::new(HashMap::new()),
        })
    }

    /// Hand the receiver half of `session_id`'s `WsEvent` channel to
    /// the caller (typically the WS handler). Returns `None` if the
    /// session has already been subscribed.
    ///
    /// If no channel exists yet, this creates one and stashes the
    /// sender so that a subsequent `new_port` can route events into
    /// the same channel.
    pub fn subscribe(&self, session_id: Uuid) -> Option<mpsc::UnboundedReceiver<WsEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut senders = self.event_senders.lock();
        if senders.contains_key(&session_id) {
            return None;
        }
        senders.insert(session_id, tx);
        Some(rx)
    }

    /// Return the sender for `session_id`, creating the channel if
    /// necessary. Used internally by `new_port` so that ports
    /// allocated in a session that hasn't yet called `subscribe`
    /// still get a (potentially undelivered) channel.
    fn sender_for(&self, session_id: Uuid) -> mpsc::UnboundedSender<WsEvent> {
        let mut senders = self.event_senders.lock();
        senders
            .entry(session_id)
            .or_insert_with(|| mpsc::unbounded_channel().0)
            .clone()
    }

    /// Allocate a new port for `session_id`.
    ///
    /// `type_str` selects the wire protocol (`tcp` / `udp` /
    /// `tcp_ssl`). `port` is the requested port number — if `Some`,
    /// the caller must also pass the correct [`PORT_PIN_TOKEN`];
    /// otherwise the request is silently downgraded to a random
    /// allocation.
    pub async fn new_port(
        &self,
        session_id: Uuid,
        type_str: &str,
        port: Option<u16>,
        token: Option<&str>,
    ) -> Result<PortNumber, AppError> {
        let kind = parse_port_type(type_str)?;

        // If a specific port is requested, the caller must present the
        // pin token. Otherwise we silently fall back to "random port".
        let requested_port = match port {
            Some(p) if token == Some(PORT_PIN_TOKEN) => Some(p),
            Some(_) => None,
            None => None,
        };

        let port = match requested_port {
            Some(p) => self.pool.take(p)?,
            None => self.pool.take_random()?,
        };

        let events = self.sender_for(session_id);
        let entity = match self.factory.create(port, kind, events).await {
            Ok(e) => e,
            Err(e) => {
                // Release the port back to the pool on bind failure.
                let _ = self.pool.recycle(port);
                return Err(e);
            }
        };

        {
            let mut sessions = self.sessions.lock();
            sessions
                .entry(session_id)
                .or_default()
                .push(SessionEntry { port, kind, entity });
        }
        self.port_to_session.lock().insert(port, session_id);
        metrics::on_new_port(wire_label(kind));
        Ok(port)
    }

    /// Shut down every port owned by `session_id`, recycling the
    /// port numbers back into the pool and dropping the per-session
    /// event sender (which causes the WS handler's `Receiver` to
    /// close).
    pub async fn close_session(&self, session_id: Uuid) {
        let entries = self.sessions.lock().remove(&session_id);
        if let Some(entries) = entries {
            let mut p2s = self.port_to_session.lock();
            for entry in entries {
                let _ = entry.entity.shutdown();
                let _ = self.pool.recycle(entry.port);
                p2s.remove(&entry.port);
                metrics::on_close_port(wire_label(entry.kind));
            }
        }
        // Drop the sender so the WS handler's receiver returns None.
        self.event_senders.lock().remove(&session_id);
    }

    /// Locate the entity that owns `client` and push `data` to it.
    ///
    /// Searches all entities in all sessions — O(n) in the number of
    /// currently-bound ports, but each probe is just a `HashMap`
    /// lookup inside the entity.
    pub async fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError> {
        let entities: Vec<Arc<dyn PortEntity>> = {
            let sessions = self.sessions.lock();
            sessions
                .values()
                .flat_map(|v| v.iter())
                .map(|e| e.entity.clone())
                .collect()
        };
        for entity in entities {
            if entity.send(client, data).is_ok() {
                return Ok(());
            }
        }
        Err(AppError::UnknownClient(client.to_string()))
    }

    /// Locate the entity that owns `client` and close that single
    /// connection.
    pub async fn close_client(&self, client: ClientId) -> Result<(), AppError> {
        let entities: Vec<Arc<dyn PortEntity>> = {
            let sessions = self.sessions.lock();
            sessions
                .values()
                .flat_map(|v| v.iter())
                .map(|e| e.entity.clone())
                .collect()
        };
        for entity in entities {
            if entity.close_client(client).is_ok() {
                return Ok(());
            }
        }
        Err(AppError::UnknownClient(client.to_string()))
    }

    /// Update the per-session broadcast flag. Currently a no-op stub
    /// — once the broadcast feature is implemented, this is where
    /// the per-session policy will be stashed.
    pub fn set_broadcast(&self, _session_id: Uuid, _b: bool) {
        // TODO: integrate with entity broadcast config.
    }
}

#[async_trait]
impl WsPortOps for PortService {
    async fn new_port(
        &self,
        session: Uuid,
        type_str: &str,
        port: Option<u16>,
        token: Option<&str>,
    ) -> Result<u16, AppError> {
        PortService::new_port(self, session, type_str, port, token)
            .await
    }

    async fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError> {
        PortService::send(self, client, data).await
    }

    async fn close_client(&self, client: ClientId) -> Result<(), AppError> {
        PortService::close_client(self, client).await
    }

    fn set_broadcast(&self, session: Uuid, b: bool) {
        PortService::set_broadcast(self, session, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::port_pool::PortPool;
    use mockall::mock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    mock! {
        pub Pool {}
        impl PortPool for Pool {
            fn take_random(&self) -> Result<u16, AppError>;
            fn take(&self, port: u16) -> Result<u16, AppError>;
            fn recycle(&self, port: u16) -> bool;
            fn used(&self) -> usize;
            fn available(&self) -> usize;
            fn capacity(&self) -> usize;
        }
    }

    mock! {
        pub Entity {}
        impl PortEntity for Entity {
            fn port(&self) -> u16;
            fn kind(&self) -> PortType;
            fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError>;
            fn close_client(&self, client: ClientId) -> Result<(), AppError>;
            fn shutdown(&self) -> Result<(), AppError>;
        }
    }

    mock! {
        pub Factory {}
        #[async_trait::async_trait]
        impl EntityFactory for Factory {
            async fn create(
                &self,
                port: u16,
                kind: PortType,
                events: mpsc::UnboundedSender<WsEvent>,
            ) -> Result<Arc<dyn PortEntity>, AppError>;
        }
    }

    fn new_entity(port: u16, kind: PortType) -> Arc<dyn PortEntity> {
        new_entity_with(port, kind, None)
    }

    fn new_entity_with(
        port: u16,
        kind: PortType,
        owns: Option<ClientId>,
    ) -> Arc<dyn PortEntity> {
        let mut e = MockEntity::new();
        e.expect_port().return_const(port);
        e.expect_kind().return_const(kind);
        e.expect_send().returning(move |c, _| match owns {
            Some(target) if c == target => Ok(()),
            _ => Err(AppError::UnknownClient("mock".into())),
        });
        e.expect_close_client().returning(move |c| match owns {
            Some(target) if c == target => Ok(()),
            _ => Err(AppError::UnknownClient("mock".into())),
        });
        e.expect_shutdown().returning(|| Ok(()));
        Arc::new(e)
    }

    fn factory_returning(entity: Arc<dyn PortEntity>) -> Arc<dyn EntityFactory> {
        let mut f = MockFactory::new();
        f.expect_create()
            .returning(move |_p, _k, _tx| Ok(entity.clone()));
        Arc::new(f)
    }

    fn factory_returning_with_calls(
        entity: Arc<dyn PortEntity>,
        calls: Arc<AtomicUsize>,
    ) -> Arc<dyn EntityFactory> {
        let mut f = MockFactory::new();
        f.expect_create().returning(move |_p, _k, _tx| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(entity.clone())
        });
        Arc::new(f)
    }

    fn factory_failing() -> Arc<dyn EntityFactory> {
        let mut f = MockFactory::new();
        f.expect_create()
            .returning(|_p, _k, _tx| Err(AppError::BadToken));
        Arc::new(f)
    }

    fn factory_no_calls() -> Arc<dyn EntityFactory> {
        // Use a never-expected handle; if `create` is invoked the test
        // panics, which is exactly the assertion we want.
        let f = MockFactory::new();
        Arc::new(f)
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(f)
    }

    // ------------------------------------------------------------------
    // new_port
    // ------------------------------------------------------------------

    #[test]
    fn new_port_random_allocates_via_take_random() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(2000_u16));
        let pool: SharedPortPool = Arc::new(pool);

        let factory_calls = Arc::new(AtomicUsize::new(0));
        let entity = new_entity(2000, PortType::Tcp);
        let factory = factory_returning_with_calls(entity, factory_calls.clone());

        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();

        let result = block_on(svc.new_port(session, "tcp", None, None));
        assert_eq!(result.unwrap(), 2000);
        assert_eq!(factory_calls.load(Ordering::SeqCst), 1);
        // Metrics emission is exercised by the production code path;
        // verifying the recorder side-channel is done in metrics::tests.
    }

    #[test]
    fn new_port_specific_with_correct_token_uses_take() {
        let mut pool = MockPool::new();
        pool.expect_take()
            .withf(|&p| p == 8080)
            .times(1)
            .returning(|_| Ok(8080_u16));
        let pool: SharedPortPool = Arc::new(pool);

        let entity = new_entity(8080, PortType::Udp);
        let factory = factory_returning(entity);

        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        let res = block_on(svc.new_port(session, "udp", Some(8080), Some(PORT_PIN_TOKEN)));
        assert_eq!(res.unwrap(), 8080);
    }

    #[test]
    fn new_port_specific_with_wrong_token_falls_back_to_random() {
        let mut pool = MockPool::new();
        // take should NOT be called when the token is wrong.
        pool.expect_take().times(0);
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(4321_u16));
        let pool: SharedPortPool = Arc::new(pool);

        let entity = new_entity(4321, PortType::Tcp);
        let factory = factory_returning(entity);

        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        let res = block_on(svc.new_port(session, "tcp", Some(8080), Some("wrong")));
        assert_eq!(res.unwrap(), 4321);
    }

    #[test]
    fn new_port_factory_failure_recycles_port() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(2500_u16));
        pool.expect_recycle()
            .withf(|&p| p == 2500)
            .times(1)
            .returning(|_| true);
        let pool: SharedPortPool = Arc::new(pool);

        let factory = factory_failing();
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        let res = block_on(svc.new_port(session, "tcp", None, None));
        assert!(matches!(res, Err(AppError::BadToken)));
    }

    #[test]
    fn new_port_no_port_available_propagates() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Err(AppError::NoPortAvailable));
        let pool: SharedPortPool = Arc::new(pool);

        let factory = factory_no_calls();
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        let res = block_on(svc.new_port(session, "tcp", None, None));
        assert!(matches!(res, Err(AppError::NoPortAvailable)));
    }

    #[test]
    fn new_port_bad_type_propagates_parse_error() {
        // Pool is untouched, factory is untouched.
        let pool: SharedPortPool = Arc::new(MockPool::new());
        let factory = factory_no_calls();
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        let res = block_on(svc.new_port(session, "http", None, None));
        match res {
            Err(AppError::BadType(s)) => assert_eq!(s, "http"),
            other => panic!("expected BadType(\"http\"), got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // close_session
    // ------------------------------------------------------------------

    #[test]
    fn close_session_shuts_down_all_entities_and_recycles_ports() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(2)
            .returning(|| Ok(3000_u16));
        pool.expect_recycle()
            .withf(|&p| p == 3000)
            .times(2)
            .returning(|_| true);
        let pool: SharedPortPool = Arc::new(pool);

        let entity = new_entity(3000, PortType::Tcp);
        let factory = factory_returning(entity);
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        block_on(svc.new_port(session, "tcp", None, None)).unwrap();
        block_on(svc.new_port(session, "tcp", None, None)).unwrap();

        // No panic and pool.recycle was called twice (asserted via mockall).
        block_on(svc.close_session(session));
    }

    // ------------------------------------------------------------------
    // send / close_client
    // ------------------------------------------------------------------

    #[test]
    fn send_returns_unknown_client_when_no_entity_has_it() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(4000_u16));
        let pool: SharedPortPool = Arc::new(pool);

        let entity = new_entity(4000, PortType::Tcp);
        let factory = factory_returning(entity);
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        block_on(svc.new_port(session, "tcp", None, None)).unwrap();

        let bogus = ClientId::new_v4();
        let res = block_on(svc.send(bogus, b"hi"));
        assert!(matches!(res, Err(AppError::UnknownClient(_))));
    }

    #[test]
    fn send_finds_entity_that_owns_the_client() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(5000_u16));
        let pool: SharedPortPool = Arc::new(pool);

        let target = ClientId::new_v4();
        let entity = new_entity_with(5000, PortType::Tcp, Some(target));
        let factory = factory_returning(entity);
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        block_on(svc.new_port(session, "tcp", None, None)).unwrap();

        let res = block_on(svc.send(target, b"hello"));
        assert!(res.is_ok());
    }

    #[test]
    fn close_client_returns_unknown_client_when_absent() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(6000_u16));
        let pool: SharedPortPool = Arc::new(pool);

        let entity = new_entity(6000, PortType::Tcp);
        let factory = factory_returning(entity);
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        block_on(svc.new_port(session, "tcp", None, None)).unwrap();

        let bogus = ClientId::new_v4();
        let res = block_on(svc.close_client(bogus));
        assert!(matches!(res, Err(AppError::UnknownClient(_))));
    }

    // ------------------------------------------------------------------
    // subscribe
    // ------------------------------------------------------------------

    #[test]
    fn subscribe_returns_receiver_then_none_on_second_call() {
        let pool: SharedPortPool = Arc::new(MockPool::new());
        let factory = factory_no_calls();
        let svc = PortService::new(pool, factory);
        let session = Uuid::new_v4();
        assert!(svc.subscribe(session).is_some());
        assert!(svc.subscribe(session).is_none());
    }

    // ------------------------------------------------------------------
    // WsPortOps delegation
    // ------------------------------------------------------------------

    #[test]
    fn ws_port_ops_delegates_to_inherent_methods() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(7000_u16));
        let pool: SharedPortPool = Arc::new(pool);

        let entity = new_entity(7000, PortType::Tcp);
        let factory = factory_returning(entity);
        let svc = PortService::new(pool, factory);

        let ops: Arc<dyn WsPortOps> = svc.clone();
        let session = Uuid::new_v4();
        // Use UFCS to disambiguate from the inherent `new_port` method
        // (same name, different receiver type).
        let res = block_on(WsPortOps::new_port(
            ops.as_ref(),
            session,
            "tcp",
            None,
            None,
        ));
        assert_eq!(res.unwrap(), 7000);

        // Unknown client path is also reachable through the trait.
        let res = block_on(WsPortOps::send(ops.as_ref(), ClientId::new_v4(), b"x"));
        assert!(matches!(res, Err(AppError::UnknownClient(_))));
    }
}
