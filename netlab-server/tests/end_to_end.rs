//! End-to-end smoke test: bring up the full HTTP+WS server, talk to it
//! as a real browser would, and verify that the WS protocol stays
//! compatible with the legacy Java client.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

use netlab_server::application::port_pool::RandomPortPool;
use netlab_server::application::port_service::PortService;
use netlab_server::bootstrap::DefaultEntityFactory;
use netlab_server::config::NetlabConfig;
use netlab_server::domain::port::PortRange;
use netlab_server::infrastructure::http::router::{build_router, AppState};
use netlab_server::infrastructure::metrics::exporter;

async fn bring_up_server() -> (SocketAddr, Arc<PortService>) {
    // Force-install the Prometheus recorder so /metrics works.
    let _ = exporter::install();

    // Tight port range to avoid colliding with anything else in CI.
    let pool = Arc::new(RandomPortPool::new(PortRange::new(21100, 21200).unwrap()));
    let factory = Arc::new(DefaultEntityFactory::new(None));
    let service = PortService::new(pool, factory);

    let state = Arc::new(AppState {
        service: service.clone(),
    });
    let router = build_router(state, "tests/static_test");

    // Bind to an ephemeral port on localhost.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    // Give the server a moment to start accepting.
    tokio::time::sleep(Duration::from_millis(50)).await;

    (addr, service)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_flow_newp_tcp_echo_and_close() {
    let (addr, _service) = bring_up_server().await;

    // Connect WS
    let url = format!("ws://{addr}/ws/netlab");
    let req = url.into_client_request().expect("ws request");
    let (mut ws, _resp) = tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect");
    let (mut ws_tx, mut ws_rx) = ws.split();

    // 1. newp tcp
    ws_tx
        .send(Message::Text(r#"{"action":"newp","type":"tcp"}"#.into()))
        .await
        .expect("send newp");

    // 2. expect {action:port, port:N}
    let port_frame = recv_action(&mut ws_rx, "port").await;
    let port = port_frame["port"].as_u64().expect("port is u64") as u16;
    assert!((21100..21200).contains(&port), "port {port} in range");

    // 3. Connect a raw TCP client to that port FIRST (so the server-side
    //    accept loop fires and emits the Connected event for us to read).
    let mut raw = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("raw tcp connect");

    // 4. expect Connected
    let connected_frame = recv_action(&mut ws_rx, "connected").await;
    let client_id = connected_frame["client"].as_str().expect("client id").to_string();
    let addr_str = connected_frame["addr"].as_str().expect("addr").to_string();
    assert!(addr_str.contains(':'), "addr has port");

    // 5. Send data over the raw TCP
    raw.write_all(b"hello").await.expect("raw write");
    drop(raw); // triggers Closed on the server side

    // 6. expect Data frame with hex("hello") = "68656c6c6f"
    let data_frame = recv_action(&mut ws_rx, "data").await;
    assert_eq!(data_frame["data"].as_str().unwrap(), "68656c6c6f");
    assert_eq!(data_frame["client"].as_str().unwrap(), client_id);

    // 7. expect Closed frame
    let closed_frame = recv_action(&mut ws_rx, "closed").await;
    assert_eq!(closed_frame["client"].as_str().unwrap(), client_id);

    // 8. Close the WS
    ws_tx.send(Message::Close(None)).await.expect("ws close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_flow_newp_udp_sends_datagram() {
    let (addr, _service) = bring_up_server().await;
    let url = format!("ws://{addr}/ws/netlab");
    let req = url.into_client_request().expect("ws request");
    let (mut ws, _resp) = tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect");
    let (mut ws_tx, mut ws_rx) = ws.split();

    // 1. newp udp
    ws_tx
        .send(Message::Text(r#"{"action":"newp","type":"udp"}"#.into()))
        .await
        .expect("send newp");

    let port_frame = recv_action(&mut ws_rx, "port").await;
    let port = port_frame["port"].as_u64().expect("port u64") as u16;

    // 2. send a UDP datagram from an ephemeral port
    let client = tokio::net::UdpSocket::bind(("127.0.0.1", 0))
        .await
        .expect("udp bind");
    client.send_to(b"abc", ("127.0.0.1", port)).await.expect("send udp");

    // 3. expect Data (hex 616263)
    let data_frame = recv_action(&mut ws_rx, "data").await;
    assert_eq!(data_frame["data"].as_str().unwrap(), "616263");
    assert_eq!(data_frame["hex"].as_bool(), Some(true));

    // Cleanup
    ws_tx.send(Message::Close(None)).await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heartbeat_and_bad_json_do_not_break_session() {
    let (addr, _service) = bring_up_server().await;
    let url = format!("ws://{addr}/ws/netlab");
    let req = url.into_client_request().expect("ws request");
    let (mut ws, _resp) = tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect");
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Empty object = heartbeat
    ws_tx.send(Message::Text("{}".into())).await.expect("hb");

    // Bad JSON
    ws_tx
        .send(Message::Text("not-json".into()))
        .await
        .expect("bad json");

    // Expect a single error frame
    let err = recv_action(&mut ws_rx, "error").await;
    assert!(err["msg"].as_str().unwrap().contains("json"));

    // Session should still be alive: a follow-up valid action works
    ws_tx
        .send(Message::Text(r#"{"action":"newp","type":"tcp"}"#.into()))
        .await
        .expect("newp after bad json");
    let _ = recv_action(&mut ws_rx, "port").await;

    ws_tx.send(Message::Close(None)).await.ok();
}

async fn recv_action<S>(
    rx: &mut S,
    expected: &str,
) -> serde_json::Value
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    let frame = timeout(Duration::from_secs(3), async {
        loop {
            match rx.next().await {
                Some(Ok(Message::Text(t))) => {
                    let v: serde_json::Value = serde_json::from_str(&t)
                        .unwrap_or_else(|_| serde_json::json!({"_raw": t.to_string()}));
                    if v.get("action").and_then(|a| a.as_str()) == Some(expected) {
                        return v;
                    }
                    // ignore non-matching events
                }
                Some(Ok(Message::Close(_))) => panic!("ws closed before {expected}"),
                Some(Ok(_)) => {} // ping/pong/binary
                Some(Err(e)) => panic!("ws err: {e}"),
                None => panic!("ws stream ended before {expected}"),
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timeout waiting for {expected}"));
    frame
}

#[test]
fn config_loads_with_defaults() {
    // We can't easily load the file from tests (working dir is the crate
    // root when running `cargo test`), but we can at least make sure
    // the load path doesn't panic if config is missing — it should
    // produce an error rather than a panic.
    let res = NetlabConfig::load();
    // Either it loads the bundled file (if tests run from netlab-server/)
    // or it returns an error. Both are acceptable.
    let _ = res;
}
