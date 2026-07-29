//! WebSocket message dispatch.
//!
//! One entry point: [`handle`]. It parses the incoming JSON
//! envelope, routes by `action`, calls the corresponding operation
//! on the injected [`PortService`] reference, and returns the JSON
//! frames that should be written back to the WS peer.

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::application::port_service::PortService;
use crate::domain::client::ClientId;
use crate::domain::errors::AppError;

/// Reserved for the keep-alive "empty object" frame; no operation
/// is triggered.
pub const HEARTBEAT_ACTION: &str = "";
/// Allocate a new port (`{"action":"newp",...}`).
pub const ACTION_NEWP: &str = "newp";
/// Send a payload to a connected client (`{"action":"sendc",...}`).
pub const ACTION_SENDC: &str = "sendc";
/// Close a single client connection (`{"action":"closec",...}`).
pub const ACTION_CLOSEC: &str = "closec";
/// Update per-session configuration (`{"action":"config",...}`).
pub const ACTION_CONFIG: &str = "config";

/// Inbound message envelope. All fields are optional — different
/// actions need different subsets. Unknown fields are silently
/// ignored.
#[derive(Debug, Deserialize)]
pub struct Inbound {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub port: Option<i32>,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub hex: Option<bool>,
    #[serde(default)]
    pub broadcast: Option<bool>,
}

/// Standard error frame.
pub fn make_error(msg: impl Into<String>) -> Value {
    serde_json::json!({"action":"error","msg":msg.into()})
}

/// Standard `port` frame returned after a successful `newp`.
pub fn make_port(port: u16) -> Value {
    serde_json::json!({"action":"port","port":port})
}

/// Process one inbound WS text frame, returning the frames to
/// write back. Most actions produce 0 or 1 frame.
///
/// Takes `&PortService` (not a trait object) so the call site in
/// `infrastructure::ws::handler` can pass `&state.service` where
/// `state.service: Arc<PortService>` — deref coercion makes that
/// work transparently.
pub async fn handle(service: &PortService, session_id: Uuid, text: &str) -> Vec<Value> {
    let inbound: Inbound = match serde_json::from_str(text) {
        Ok(i) => i,
        Err(_) => return vec![make_error("bad json")],
    };

    // Heartbeat: an object with no `action` is a keep-alive ping.
    if inbound.action.is_empty() {
        return vec![];
    }

    match inbound.action.as_str() {
        ACTION_NEWP => {
            let type_str = inbound.r#type.as_deref().unwrap_or("");
            let port = inbound
                .port
                .and_then(|p| if p < 0 { None } else { Some(p as u16) });
            let token = inbound.token.as_deref();
            match service.new_port(session_id, type_str, port, token).await {
                Ok(p) => vec![make_port(p)],
                Err(e) => vec![make_error(e.to_string())],
            }
        }
        ACTION_SENDC => {
            let client_str = match inbound.client.as_deref() {
                Some(s) if !s.is_empty() => s,
                _ => return vec![make_error("missing client")],
            };
            let client = match Uuid::parse_str(client_str) {
                Ok(u) => ClientId::from(u),
                Err(_) => {
                    return vec![make_error(format!("bad client uuid: {client_str}"))];
                }
            };
            let data = match inbound.data.as_deref() {
                Some(d) => d,
                None => return vec![make_error("missing data")],
            };
            let bytes: Vec<u8> = if inbound.hex.unwrap_or(false) {
                match hex::decode(data) {
                    Ok(b) => b,
                    Err(_) => return vec![make_error(AppError::BadHex.to_string())],
                }
            } else {
                data.as_bytes().to_vec()
            };
            match service.send(client, &bytes).await {
                Ok(()) => vec![],
                Err(e) => vec![make_error(e.to_string())],
            }
        }
        ACTION_CLOSEC => {
            let client_str = match inbound.client.as_deref() {
                Some(s) if !s.is_empty() => s,
                _ => return vec![make_error("missing client")],
            };
            let client = match Uuid::parse_str(client_str) {
                Ok(u) => ClientId::from(u),
                Err(_) => {
                    return vec![make_error(format!("bad client uuid: {client_str}"))];
                }
            };
            match service.close_client(client).await {
                Ok(()) => vec![],
                Err(e) => vec![make_error(e.to_string())],
            }
        }
        ACTION_CONFIG => {
            service.set_broadcast(session_id, inbound.broadcast.unwrap_or(false));
            vec![]
        }
        other => vec![make_error(format!("unknown action {other}"))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::port_pool::PortPool;
    use crate::application::port_service::{EntityFactory, PORT_PIN_TOKEN};
    use crate::domain::port_entity::WsEvent;
    use async_trait::async_trait;
    use mockall::mock;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    // ------------------------------------------------------------------
    // Mocks
    // ------------------------------------------------------------------

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
        impl crate::domain::port_entity::PortEntity for Entity {
            fn port(&self) -> u16;
            fn kind(&self) -> crate::domain::port_entity::PortType;
            fn send(&self, client: ClientId, data: &[u8]) -> Result<(), AppError>;
            fn close_client(&self, client: ClientId) -> Result<(), AppError>;
            fn shutdown(&self) -> Result<(), AppError>;
        }
    }

    // EntityFactory is async — we wrap a mock in a tiny adapter so the
    // test doesn't have to plumb `async_trait` through mockall twice.
    struct StaticFactory {
        entity: Arc<dyn crate::domain::port_entity::PortEntity>,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl EntityFactory for StaticFactory {
        async fn create(
            &self,
            _port: u16,
            _kind: crate::domain::port_entity::PortType,
            _events: mpsc::UnboundedSender<WsEvent>,
        ) -> Result<Arc<dyn crate::domain::port_entity::PortEntity>, AppError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.entity.clone())
        }
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(f)
    }

    fn new_entity(port: u16) -> Arc<dyn crate::domain::port_entity::PortEntity> {
        let mut e = MockEntity::new();
        e.expect_port().return_const(port);
        e.expect_kind()
            .return_const(crate::domain::port_entity::PortType::Tcp);
        e.expect_send()
            .returning(|_, _| Err(AppError::UnknownClient("mock".into())));
        e.expect_close_client()
            .returning(|_| Err(AppError::UnknownClient("mock".into())));
        e.expect_shutdown().returning(|| Ok(()));
        Arc::new(e)
    }

    fn build_service(
        pool: MockPool,
        entity: Arc<dyn crate::domain::port_entity::PortEntity>,
    ) -> (Arc<PortService>, Arc<StaticFactory>) {
        let factory = Arc::new(StaticFactory {
            entity,
            calls: AtomicUsize::new(0),
        });
        let pool: Arc<dyn PortPool> = Arc::new(pool);
        let svc = PortService::new(pool, factory.clone());
        (svc, factory)
    }

    fn handle_sync(svc: &PortService, session: Uuid, text: &str) -> Vec<Value> {
        block_on(handle(svc, session, text))
    }

    // ------------------------------------------------------------------
    // Tests
    // ------------------------------------------------------------------

    #[test]
    fn heartbeat_empty_object_yields_no_frames() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let out = handle_sync(&svc, Uuid::new_v4(), "{}");
        assert!(out.is_empty(), "expected no frames, got {out:?}");
    }

    #[test]
    fn newp_ok_returns_port_frame() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(2000_u16));
        let (svc, factory) = build_service(pool, new_entity(2000));
        let out = handle_sync(&svc, Uuid::new_v4(), r#"{"action":"newp","type":"tcp"}"#);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "port");
        assert_eq!(out[0]["port"], 2000);
        assert_eq!(factory.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn newp_passes_pinned_port_and_token_through() {
        let mut pool = MockPool::new();
        pool.expect_take()
            .withf(|&p| p == 8080)
            .times(1)
            .returning(|_| Ok(8080_u16));
        let (svc, _) = build_service(pool, new_entity(8080));
        let out = handle_sync(
            &svc,
            Uuid::new_v4(),
            r#"{"action":"newp","type":"tcp","port":8080,"token":"LuatOS-NetLab"}"#,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["port"], 8080);
    }

    #[test]
    fn newp_negative_port_is_treated_as_random() {
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(1234_u16));
        let (svc, _) = build_service(pool, new_entity(1234));
        let out = handle_sync(
            &svc,
            Uuid::new_v4(),
            r#"{"action":"newp","type":"tcp","port":-1}"#,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["port"], 1234);
    }

    #[test]
    fn newp_bad_type_returns_error() {
        let pool = MockPool::new();
        let (svc, _) = build_service(pool, new_entity(2000));
        let out = handle_sync(&svc, Uuid::new_v4(), r#"{"action":"newp","type":"http"}"#);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "error");
        assert!(out[0]["msg"].as_str().unwrap().contains("http"));
    }

    #[test]
    fn sendc_hex_decodes_payload() {
        let target = ClientId::new_v4();
        let target_str = target.to_string();
        let captured: Arc<std::sync::Mutex<Vec<u8>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

        // Entity that records what it was asked to `send`.
        let captured_c = captured.clone();
        let target_c = target;
        let mut e = MockEntity::new();
        e.expect_port().return_const(2000_u16);
        e.expect_kind()
            .return_const(crate::domain::port_entity::PortType::Tcp);
        e.expect_send().returning(move |c, data| {
            if c == target_c {
                *captured_c.lock().unwrap() = data.to_vec();
                Ok(())
            } else {
                Err(AppError::UnknownClient("mock".into()))
            }
        });
        e.expect_close_client()
            .returning(|_| Err(AppError::UnknownClient("mock".into())));
        e.expect_shutdown().returning(|| Ok(()));
        let entity: Arc<dyn crate::domain::port_entity::PortEntity> = Arc::new(e);

        // Register the entity to a session via new_port so service.send can find it.
        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(2000_u16));
        let (svc, _) = build_service(pool, entity);
        let session = Uuid::new_v4();
        // Pre-register by allocating the port; handle() round-trip via newp.
        let _ = handle_sync(&svc, session, r#"{"action":"newp","type":"tcp"}"#);

        let text =
            format!(r#"{{"action":"sendc","client":"{target_str}","data":"AABB","hex":true}}"#);
        let out = handle_sync(&svc, session, &text);
        assert!(out.is_empty(), "expected no frames, got {out:?}");
        let bytes = captured.lock().unwrap();
        assert_eq!(*bytes, vec![0xAA, 0xBB]);
    }

    #[test]
    fn sendc_plain_uses_utf8_bytes() {
        let target = ClientId::new_v4();
        let target_str = target.to_string();
        let captured: Arc<std::sync::Mutex<Vec<u8>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

        let captured_c = captured.clone();
        let target_c = target;
        let mut e = MockEntity::new();
        e.expect_port().return_const(2000_u16);
        e.expect_kind()
            .return_const(crate::domain::port_entity::PortType::Tcp);
        e.expect_send().returning(move |c, data| {
            if c == target_c {
                *captured_c.lock().unwrap() = data.to_vec();
                Ok(())
            } else {
                Err(AppError::UnknownClient("mock".into()))
            }
        });
        e.expect_close_client()
            .returning(|_| Err(AppError::UnknownClient("mock".into())));
        e.expect_shutdown().returning(|| Ok(()));
        let entity: Arc<dyn crate::domain::port_entity::PortEntity> = Arc::new(e);

        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(2000_u16));
        let (svc, _) = build_service(pool, entity);
        let session = Uuid::new_v4();
        let _ = handle_sync(&svc, session, r#"{"action":"newp","type":"tcp"}"#);

        let text =
            format!(r#"{{"action":"sendc","client":"{target_str}","data":"hi","hex":false}}"#);
        let out = handle_sync(&svc, session, &text);
        assert!(out.is_empty(), "expected no frames, got {out:?}");
        let bytes = captured.lock().unwrap();
        assert_eq!(*bytes, b"hi".to_vec());
    }

    #[test]
    fn sendc_bad_hex_returns_error() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let target = ClientId::new_v4();
        let text = format!(r#"{{"action":"sendc","client":"{target}","data":"zz","hex":true}}"#);
        let out = handle_sync(&svc, Uuid::new_v4(), &text);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "error");
        assert!(out[0]["msg"].as_str().unwrap().contains("hex"));
    }

    #[test]
    fn sendc_missing_client_returns_error() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let out = handle_sync(&svc, Uuid::new_v4(), r#"{"action":"sendc","data":"hi"}"#);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "error");
    }

    #[test]
    fn sendc_bad_uuid_returns_error() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let out = handle_sync(
            &svc,
            Uuid::new_v4(),
            r#"{"action":"sendc","client":"not-a-uuid","data":"hi"}"#,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "error");
    }

    #[test]
    fn closec_returns_no_frames_on_success() {
        let target = ClientId::new_v4();
        let target_c = target;
        let mut e = MockEntity::new();
        e.expect_port().return_const(2000_u16);
        e.expect_kind()
            .return_const(crate::domain::port_entity::PortType::Tcp);
        e.expect_send()
            .returning(|_, _| Err(AppError::UnknownClient("mock".into())));
        e.expect_close_client().returning(move |c| {
            if c == target_c {
                Ok(())
            } else {
                Err(AppError::UnknownClient("mock".into()))
            }
        });
        e.expect_shutdown().returning(|| Ok(()));
        let entity: Arc<dyn crate::domain::port_entity::PortEntity> = Arc::new(e);

        let mut pool = MockPool::new();
        pool.expect_take_random()
            .times(1)
            .returning(|| Ok(2000_u16));
        let (svc, _) = build_service(pool, entity);
        let session = Uuid::new_v4();
        let _ = handle_sync(&svc, session, r#"{"action":"newp","type":"tcp"}"#);
        let target_str = target.to_string();
        let text = format!(r#"{{"action":"closec","client":"{target_str}"}}"#);
        let out = handle_sync(&svc, session, &text);
        assert!(out.is_empty(), "expected no frames, got {out:?}");
    }

    #[test]
    fn closec_propagates_error() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let target = ClientId::new_v4();
        let text = format!(r#"{{"action":"closec","client":"{target}"}}"#);
        let out = handle_sync(&svc, Uuid::new_v4(), &text);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "error");
    }

    #[test]
    fn config_returns_no_frames() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let out = handle_sync(
            &svc,
            Uuid::new_v4(),
            r#"{"action":"config","broadcast":true}"#,
        );
        assert!(out.is_empty());
        let out = handle_sync(&svc, Uuid::new_v4(), r#"{"action":"config"}"#);
        assert!(out.is_empty());
    }

    #[test]
    fn bad_json_returns_error_frame() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let out = handle_sync(&svc, Uuid::new_v4(), "not json");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "error");
        assert_eq!(out[0]["msg"], "bad json");
    }

    #[test]
    fn unknown_action_returns_error_frame() {
        let (svc, _) = build_service(MockPool::new(), new_entity(2000));
        let out = handle_sync(&svc, Uuid::new_v4(), r#"{"action":"foo"}"#);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["action"], "error");
        assert!(out[0]["msg"].as_str().unwrap().contains("foo"));
    }

    // ------------------------------------------------------------------
    // Misc
    // ------------------------------------------------------------------

    #[test]
    fn action_constants_are_stable() {
        assert_eq!(ACTION_NEWP, "newp");
        assert_eq!(ACTION_SENDC, "sendc");
        assert_eq!(ACTION_CLOSEC, "closec");
        assert_eq!(ACTION_CONFIG, "config");
        assert_eq!(HEARTBEAT_ACTION, "");
    }

    #[test]
    fn port_pin_token_matches_documented_value() {
        assert_eq!(PORT_PIN_TOKEN, "LuatOS-NetLab");
    }
}
