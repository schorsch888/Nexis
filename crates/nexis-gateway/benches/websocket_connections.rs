use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use futures::{SinkExt, StreamExt};
use nexis_gateway::build_routes;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

async fn spawn_server() -> (String, tokio::task::JoinHandle<()>) {
    let app = build_routes();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind benchmark listener");
    let addr = listener.local_addr().expect("read local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve benchmark websocket app");
    });

    (format!("ws://{addr}/ws"), handle)
}

fn benchmark_websocket_connections(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    let mut group = c.benchmark_group("websocket/connections");

    for concurrent in [1usize, 5, 20] {
        group.throughput(Throughput::Elements(concurrent as u64));
        group.bench_function(BenchmarkId::new("connect_echo", concurrent), |b| {
            b.iter(|| {
                runtime.block_on(async {
                    let (url, handle) = spawn_server().await;

                    let tasks = (0..concurrent)
                        .map(|idx| {
                            let url = url.clone();
                            tokio::spawn(async move {
                                let (mut ws, _resp) =
                                    connect_async(&url).await.expect("websocket should connect");
                                ws.send(Message::Text(format!("ping-{idx}").into()))
                                    .await
                                    .expect("send benchmark frame");

                                let reply = ws
                                    .next()
                                    .await
                                    .expect("reply should exist")
                                    .expect("reply should be valid websocket message");

                                match reply {
                                    Message::Text(text) => assert_eq!(text, format!("ping-{idx}")),
                                    other => panic!("unexpected websocket reply: {other:?}"),
                                }
                            })
                        })
                        .collect::<Vec<_>>();

                    for task in tasks {
                        task.await.expect("websocket task should complete");
                    }

                    handle.abort();
                });
            });
        });
    }

    group.finish();
}

criterion_group!(websocket_benches, benchmark_websocket_connections);
criterion_main!(websocket_benches);
