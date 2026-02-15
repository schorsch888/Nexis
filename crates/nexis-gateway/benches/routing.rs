use axum::body::Body;
use axum::http::{Request, StatusCode};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nexis_gateway::build_routes;
use serde_json::Value;
use tower::ServiceExt;

fn health_request() -> Request<Body> {
    Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("health request should build")
}

fn create_room_request() -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/rooms")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "bench-room",
                "topic": "benchmark",
            })
            .to_string(),
        ))
        .expect("create-room request should build")
}

fn send_message_request(room_id: &str, index: usize) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "roomId": room_id,
                "sender": "bench",
                "text": format!("message-{index}"),
            })
            .to_string(),
        ))
        .expect("send-message request should build")
}

fn benchmark_health_latency(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");
    let app = build_routes();

    c.bench_function("routing/health_latency", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let response = app
                    .clone()
                    .oneshot(health_request())
                    .await
                    .expect("health route should respond");
                assert_eq!(response.status(), StatusCode::OK);
            });
        });
    });
}

fn benchmark_create_room_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");
    let app = build_routes();

    let mut group = c.benchmark_group("routing/create_room");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("throughput", "single"), |b| {
        b.iter(|| {
            runtime.block_on(async {
                let response = app
                    .clone()
                    .oneshot(create_room_request())
                    .await
                    .expect("create-room route should respond");
                assert_eq!(response.status(), StatusCode::CREATED);
            });
        });
    });
    group.finish();
}

fn benchmark_send_message_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");
    let app = build_routes();

    let room_id = runtime.block_on(async {
        let response = app
            .clone()
            .oneshot(create_room_request())
            .await
            .expect("setup create-room route should respond");
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("setup response body should read");
        let payload: Value = serde_json::from_slice(&body).expect("setup payload should parse");
        payload["id"]
            .as_str()
            .expect("setup payload should contain room id")
            .to_string()
    });

    let mut group = c.benchmark_group("routing/send_message");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("throughput", "single_room"), |b| {
        let mut index = 0usize;
        b.iter(|| {
            runtime.block_on(async {
                let response = app
                    .clone()
                    .oneshot(send_message_request(&room_id, index))
                    .await
                    .expect("send-message route should respond");
                assert_eq!(response.status(), StatusCode::CREATED);
                index += 1;
            });
        });
    });
    group.finish();
}

criterion_group!(
    routing_benches,
    benchmark_health_latency,
    benchmark_create_room_throughput,
    benchmark_send_message_throughput
);
criterion_main!(routing_benches);
