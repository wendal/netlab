//! Integration tests for [`TcpPortEntity`]: spin up a real TCP listener,
//! drive it with `tokio::net::TcpStream`, and assert the entity emits the
//! expected `WsEvent` sequence.

use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;

use netlab_server::domain::errors::AppError;
use netlab_server::domain::port_entity::{PortEntity, PortType, WsEvent};
use netlab_server::infrastructure::tcp::entity::TcpPortEntity;

const TEST_PORT: u16 = 21099;

fn fresh_events() -> (
    mpsc::UnboundedSender<WsEvent>,
    mpsc::UnboundedReceiver<WsEvent>,
) {
    mpsc::unbounded_channel()
}

async fn recv_within(rx: &mut mpsc::UnboundedReceiver<WsEvent>, dur: Duration) -> Option<WsEvent> {
    timeout(dur, rx.recv()).await.ok().flatten()
}

#[tokio::test]
async fn tcp_connect_data_closed_sequence() {
    let (events_tx, mut events_rx) = fresh_events();

    let entity = TcpPortEntity::start(TEST_PORT, false, events_tx, None)
        .await
        .expect("start tcp entity");

    // Connect to the entity.
    let mut client = TcpStream::connect(("127.0.0.1", TEST_PORT))
        .await
        .expect("client connect");

    // 1. Connected event
    let ev = recv_within(&mut events_rx, Duration::from_secs(2))
        .await
        .expect("connected event");
    match ev {
        WsEvent::Connected { client, addr } => {
            assert!(addr.starts_with("127.0.0.1:") || addr.starts_with("[::ffff:127.0.0.1]"));
            // Save the client id for later assertions.
            let _ = client;
        }
        other => panic!("expected Connected, got {other:?}"),
    }

    // 2. Send "hello" and expect Data event with hex-encoded payload.
    client.write_all(b"hello").await.expect("client write");
    let ev = recv_within(&mut events_rx, Duration::from_secs(2))
        .await
        .expect("data event");
    match ev {
        WsEvent::Data { data, hex, .. } => {
            assert!(hex, "data should be hex-encoded");
            assert_eq!(data, "68656c6c6f");
        }
        other => panic!("expected Data, got {other:?}"),
    }

    // 3. Close the client side; expect Closed event.
    drop(client);
    let ev = recv_within(&mut events_rx, Duration::from_secs(2))
        .await
        .expect("closed event");
    assert!(matches!(ev, WsEvent::Closed { .. }), "got {ev:?}");

    // 4. Shutdown the entity.
    entity.shutdown().expect("shutdown");
}

#[tokio::test]
async fn tcp_entity_reports_kind_and_port() {
    let (events_tx, _events_rx) = fresh_events();
    let entity = TcpPortEntity::start(21100, false, events_tx, None)
        .await
        .expect("start");
    assert_eq!(entity.port(), 21100);
    assert_eq!(entity.kind(), PortType::Tcp);
    entity.shutdown().expect("shutdown");
}

#[tokio::test]
async fn send_to_unknown_client_returns_error() {
    let (events_tx, _events_rx) = fresh_events();
    let entity = TcpPortEntity::start(21101, false, events_tx, None)
        .await
        .expect("start");

    let bogus = uuid::Uuid::new_v4();
    let err = entity.send(bogus, b"hello").expect_err("unknown client");
    assert!(matches!(err, AppError::UnknownClient(_)), "got {err:?}");

    entity.shutdown().expect("shutdown");
}

#[tokio::test]
async fn tcp_listen_fails_on_port_in_use() {
    let (tx_a, _rx_a) = fresh_events();
    let _a = TcpPortEntity::start(21102, false, tx_a, None)
        .await
        .expect("first bind");

    let (tx_b, _rx_b) = fresh_events();
    let err = TcpPortEntity::start(21102, false, tx_b, None)
        .await
        .expect_err("second bind must fail");
    assert!(matches!(err, AppError::PortBind(_)), "got {err:?}");

    _a.shutdown().expect("shutdown");
}
