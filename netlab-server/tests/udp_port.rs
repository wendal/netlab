//! Integration tests for [`UdpPortEntity`].
//!
//! Spins up a real `UdpSocket` on an ephemeral port, drives a single
//! remote peer through `send_to`, and asserts that the WsEvent stream
//! produces exactly one `Connected` followed by one `Data` per datagram.

use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::timeout;

use netlab_server::domain::port_entity::{PortEntity, WsEvent};
use netlab_server::infrastructure::udp::entity::UdpPortEntity;

#[tokio::test]
async fn udp_connect_once_data_each() {
    // 1. Start UdpPortEntity on an ephemeral port (port 0).
    let (tx, mut rx) = mpsc::unbounded_channel::<WsEvent>();
    let entity = UdpPortEntity::start(0, tx).await.expect("start udp entity");
    let port = entity.port();
    assert!(port > 0, "expected ephemeral port, got {port}");

    // 2. Remote peer binds to an ephemeral port and sends "hi".
    let peer = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind peer socket");
    let peer_addr = peer.local_addr().expect("peer local addr");
    peer.send_to(b"hi", ("127.0.0.1", port))
        .await
        .expect("first send_to");

    // 3. First event must be `Connected` (and the addr must match the peer).
    let first = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for first event")
        .expect("channel closed unexpectedly");
    let client_id = match first {
        WsEvent::Connected { client, addr } => {
            assert_eq!(addr, peer_addr.to_string(), "connected addr");
            client
        }
        other => panic!("expected Connected, got {other:?}"),
    };

    // 4. Second event must be `Data` for that client ("hi" -> "6869").
    let second = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for Data")
        .expect("channel closed unexpectedly");
    match second {
        WsEvent::Data { client, data, hex } => {
            assert_eq!(client, client_id, "data client mismatch");
            assert!(hex, "udp data must be hex-encoded");
            assert_eq!(data, "6869", "expected 'hi' hex");
        }
        other => panic!("expected Data, got {other:?}"),
    }

    // 5. Send a second datagram from the same peer: must produce exactly
    //    one more `Data` event, NOT another `Connected`.
    peer.send_to(b"ho", ("127.0.0.1", port))
        .await
        .expect("second send_to");
    let third = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for second Data")
        .expect("channel closed unexpectedly");
    match third {
        WsEvent::Data { client, data, hex } => {
            assert_eq!(client, client_id);
            assert!(hex);
            assert_eq!(data, "686f", "expected 'ho' hex");
        }
        other => panic!("expected Data on second send, got {other:?}"),
    }

    // 6. Graceful shutdown.
    entity.shutdown().expect("shutdown");
}

#[tokio::test]
async fn udp_send_to_peer_roundtrip() {
    // Verifies the WS -> entity -> peer direction: send bytes via
    // `PortEntity::send` and confirm the peer actually receives them.
    let (tx, mut rx) = mpsc::unbounded_channel::<WsEvent>();
    let entity = UdpPortEntity::start(0, tx).await.expect("start udp entity");
    let port = entity.port();

    let peer = UdpSocket::bind("127.0.0.1:0").await.expect("bind peer");

    // Trigger client registration by sending one datagram from the peer.
    peer.send_to(b"ping", ("127.0.0.1", port))
        .await
        .expect("register send");

    // Drain the Connected + Data emitted by the registration send.
    match timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout Connected")
        .expect("channel closed")
    {
        WsEvent::Connected { .. } => {}
        other => panic!("expected Connected, got {other:?}"),
    }
    let client_id = match timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout Data")
        .expect("channel closed")
    {
        WsEvent::Data { client, .. } => client,
        other => panic!("expected Data, got {other:?}"),
    };

    // Send through the entity and verify the peer receives it.
    let payload = b"hello-udp";
    entity.send(client_id, payload).expect("entity send");
    let mut buf = [0u8; 64];
    let (n, from) = timeout(Duration::from_secs(2), peer.recv_from(&mut buf))
        .await
        .expect("timed out waiting for peer to receive")
        .expect("peer recv error");
    assert_eq!(from.port(), port, "received from entity port");
    assert_eq!(&buf[..n], payload, "payload mismatch");

    // Clean up.
    entity.close_client(client_id).expect("close_client");
    entity.shutdown().expect("shutdown");
}

#[tokio::test]
async fn udp_close_client_drops_registry_entry() {
    // Verifies that `close_client` removes the client but does not emit
    // any event — UDP never produces `WsEvent::Closed` (matches Java).
    let (tx, mut rx) = mpsc::unbounded_channel::<WsEvent>();
    let entity = UdpPortEntity::start(0, tx).await.expect("start udp entity");
    let port = entity.port();

    let peer = UdpSocket::bind("127.0.0.1:0").await.expect("bind peer");

    peer.send_to(b"x", ("127.0.0.1", port))
        .await
        .expect("register send");

    // Drain Connected + Data.
    let _ = timeout(Duration::from_secs(2), rx.recv()).await;
    let client_id = match timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout Data")
        .expect("channel closed")
    {
        WsEvent::Data { client, .. } => client,
        other => panic!("expected Data, got {other:?}"),
    };

    // close_client must succeed and must NOT emit any event.
    entity.close_client(client_id).expect("close_client");

    // Try to receive another event with a short timeout. There must be
    // none: UDP has no `Closed` path.
    let spurious = timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(
        spurious.is_err(),
        "close_client must not emit any WsEvent, got {:?}",
        spurious
    );

    // A subsequent send to the closed client should fail with
    // `UnknownClient`.
    let err = entity
        .send(client_id, b"after-close")
        .expect_err("send to closed client must fail");
    assert!(matches!(
        err,
        netlab_server::domain::errors::AppError::UnknownClient(_)
    ));

    entity.shutdown().expect("shutdown");
}
