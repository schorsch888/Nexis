use std::time::{Duration, Instant};

use futures::future::join_all;

#[tokio::test]
#[ignore = "stress test requires running gateway at 127.0.0.1:8080"]
async fn high_concurrency_connections() {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(256)
        .pool_idle_timeout(Duration::from_secs(60))
        .build()
        .expect("reqwest client should build");

    let concurrency = 500usize;
    let requests_per_worker = 20usize;

    wait_gateway_ready(&client).await;

    let start = Instant::now();
    let workers = (0..concurrency)
        .map(|worker| {
            let client = client.clone();
            tokio::spawn(async move {
                let mut success = 0usize;
                for i in 0..requests_per_worker {
                    let response = client
                        .post("http://127.0.0.1:8080/v1/rooms")
                        .json(&serde_json::json!({
                            "name": format!("stress-room-{worker}-{i}"),
                            "topic": "stress",
                        }))
                        .send()
                        .await
                        .expect("request should succeed");
                    if response.status().is_success() {
                        success += 1;
                    }
                }
                success
            })
        })
        .collect::<Vec<_>>();

    let results = join_all(workers).await;
    let success_total: usize = results
        .into_iter()
        .map(|result| result.expect("worker task should complete"))
        .sum();

    let elapsed = start.elapsed();
    let total_requests = concurrency * requests_per_worker;
    let throughput = total_requests as f64 / elapsed.as_secs_f64();

    assert_eq!(success_total, total_requests);
    assert!(throughput > 500.0, "throughput too low: {throughput:.2} req/s");
}

#[tokio::test]
#[ignore = "stress test requires running gateway at 127.0.0.1:8080"]
async fn message_throughput_under_load() {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(256)
        .pool_idle_timeout(Duration::from_secs(60))
        .build()
        .expect("reqwest client should build");

    wait_gateway_ready(&client).await;

    let create_resp = client
        .post("http://127.0.0.1:8080/v1/rooms")
        .json(&serde_json::json!({
            "name": "stress-throughput-room",
            "topic": "stress",
        }))
        .send()
        .await
        .expect("room create should succeed");
    assert!(create_resp.status().is_success());

    let room_payload: serde_json::Value = create_resp
        .json()
        .await
        .expect("room payload should parse");
    let room_id = room_payload["id"]
        .as_str()
        .expect("room id should exist")
        .to_string();

    let concurrency = 200usize;
    let messages_per_worker = 50usize;

    let start = Instant::now();
    let workers = (0..concurrency)
        .map(|worker| {
            let client = client.clone();
            let room_id = room_id.clone();
            tokio::spawn(async move {
                let mut success = 0usize;
                for i in 0..messages_per_worker {
                    let response = client
                        .post("http://127.0.0.1:8080/v1/messages")
                        .json(&serde_json::json!({
                            "roomId": room_id,
                            "sender": format!("sender-{worker}"),
                            "text": format!("msg-{worker}-{i}"),
                        }))
                        .send()
                        .await
                        .expect("message send should succeed");
                    if response.status().is_success() {
                        success += 1;
                    }
                }
                success
            })
        })
        .collect::<Vec<_>>();

    let results = join_all(workers).await;
    let success_total: usize = results
        .into_iter()
        .map(|result| result.expect("worker task should complete"))
        .sum();

    let elapsed = start.elapsed();
    let total_messages = concurrency * messages_per_worker;
    let throughput = total_messages as f64 / elapsed.as_secs_f64();

    assert_eq!(success_total, total_messages);
    assert!(throughput > 1_000.0, "message throughput too low: {throughput:.2} msg/s");
}

async fn wait_gateway_ready(client: &reqwest::Client) {
    for attempt in 0..30 {
        if let Ok(response) = client.get("http://127.0.0.1:8080/health").send().await {
            if response.status().is_success() {
                return;
            }
        }
        if attempt == 29 {
            panic!("gateway not ready after 30 seconds");
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
