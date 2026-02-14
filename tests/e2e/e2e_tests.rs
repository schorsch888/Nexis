//! End-to-End tests for Nexis
//!
//! These tests verify the complete system works as expected.

mod integration;

use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Test full system startup and API calls
#[tokio::test]
#[ignore = "requires running gateway"]
async fn e2e_create_room_and_send_message() {
    // Wait for gateway to be ready
    let client = reqwest::Client::new();
    
    for i in 0..10 {
        if let Ok(resp) = client.get("http://127.0.0.1:8080/health").send().await {
            if resp.status().is_success() {
                break;
            }
        }
        if i == 9 {
            panic!("Gateway not ready after 10 seconds");
        }
        sleep(Duration::from_millis(1000)).await;
    }

    // Create room
    let create_resp = client
        .post("http://127.0.0.1:8080/v1/rooms")
        .json(&serde_json::json!({
            "name": "e2e-test-room",
            "topic": "End-to-end test"
        }))
        .send()
        .await
        .expect("Failed to create room");

    assert!(create_resp.status().is_success());
    
    let room: serde_json::Value = create_resp.json().await.expect("Failed to parse room response");
    let room_id = room["id"].as_str().expect("Room ID missing");

    // Send message
    let msg_resp = client
        .post("http://127.0.0.1:8080/v1/messages")
        .json(&serde_json::json!({
            "roomId": room_id,
            "sender": "nexis:human:e2e@test.com",
            "text": "Hello from e2e test!"
        }))
        .send()
        .await
        .expect("Failed to send message");

    assert!(msg_resp.status().is_success());

    // Get room and verify message
    let get_resp = client
        .get(&format!("http://127.0.0.1:8080/v1/rooms/{}", room_id))
        .send()
        .await
        .expect("Failed to get room");

    assert!(get_resp.status().is_success());
    
    let room_data: serde_json::Value = get_resp.json().await.expect("Failed to parse room data");
    assert_eq!(room_data["messages"].as_array().unwrap().len(), 1);
    assert_eq!(room_data["messages"][0]["text"], "Hello from e2e test!");
}

/// Test WebSocket echo
#[tokio::test]
#[ignore = "requires running gateway"]
async fn e2e_websocket_echo() {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

    let (mut ws, _) = connect_async("ws://127.0.0.1:8080/ws")
        .await
        .expect("Failed to connect to WebSocket");

    ws.send(Message::Text("test-echo".to_string()))
        .await
        .expect("Failed to send message");

    let response = tokio::time::timeout(
        Duration::from_secs(5),
        ws.next()
    )
    .await
    .expect("Timeout waiting for response")
    .expect("No response received")
    .expect("Failed to receive message");

    match response {
        Message::Text(text) => assert_eq!(text, "test-echo"),
        _ => panic!("Unexpected message type"),
    }
}

/// Test concurrent room creation
#[tokio::test]
#[ignore = "requires running gateway"]
async fn e2e_concurrent_room_creation() {
    let client = reqwest::Client::new();
    
    // Wait for gateway
    for i in 0..10 {
        if let Ok(resp) = client.get("http://127.0.0.1:8080/health").send().await {
            if resp.status().is_success() {
                break;
            }
        }
        if i == 9 {
            panic!("Gateway not ready");
        }
        sleep(Duration::from_millis(1000)).await;
    }

    // Create 10 rooms concurrently
    let mut handles = vec![];
    
    for i in 0..10 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            client
                .post("http://127.0.0.1:8080/v1/rooms")
                .json(&serde_json::json!({
                    "name": format!("concurrent-room-{}", i),
                    "topic": "Concurrent test"
                }))
                .send()
                .await
                .expect("Failed to create room")
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        let response = handle.await.expect("Task failed");
        if response.status().is_success() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10, "All 10 concurrent requests should succeed");
}
