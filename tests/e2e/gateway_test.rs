//! E2E tests for Gateway WebSocket and HTTP API

use std::time::Duration;

use axum::Router;
use futures::{SinkExt, StreamExt};
use nexis_gateway::build_routes;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

async fn spawn_gateway_server() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let app: Router = build_routes();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().expect("local addr");

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve gateway app");
    });

    (addr, handle)
}

#[tokio::test]
#[ignore = "starts network listeners"]
async fn gateway_health_check() {
    let (addr, _server_handle) = spawn_gateway_server().await;

    let http = reqwest::Client::new();
    let response = http
        .get(format!("http://{}/health", addr))
        .send()
        .await
        .expect("health check request");

    assert!(response.status().is_success());
}

#[tokio::test]
#[ignore = "starts network listeners"]
async fn gateway_websocket_echo() {
    let (addr, server_handle) = spawn_gateway_server().await;

    let ws_url = format!("ws://{}/ws", addr);
    let (mut ws, _resp) = connect_async(ws_url).await.expect("connect websocket");

    ws.send(Message::Text("hello-e2e".to_string()))
        .await
        .expect("send ws message");

    let response = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("wait ws response")
        .expect("ws should produce a message")
        .expect("ws message should be ok");

    match response {
        Message::Text(text) => assert_eq!(text, "hello-e2e"),
        other => panic!("unexpected ws frame: {other:?}"),
    }

    server_handle.abort();
}
