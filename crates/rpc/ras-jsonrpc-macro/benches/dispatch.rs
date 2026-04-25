//! Criterion bench measuring per-call latency of an authenticated JSON-RPC
//! method through the full stack: generated client → axum router → handler.
//!
//! Run with `cargo bench -p ras-jsonrpc-macro`.

use criterion::{Criterion, criterion_group, criterion_main};
use ras_jsonrpc_macro::jsonrpc_service;
use ras_test_helpers::{MockAuthProvider, spawn_http};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddRequest {
    a: i64,
    b: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddResponse {
    sum: i64,
}

jsonrpc_service!({
    service_name: BenchSvc,
    openrpc: false,
    methods: [
        WITH_PERMISSIONS(["user"]) add(AddRequest) -> AddResponse,
    ]
});

fn build_router() -> axum::Router {
    BenchSvcBuilder::new("/rpc")
        .auth_provider(MockAuthProvider::default())
        .add_handler(|_user, req: AddRequest| async move { Ok(AddResponse { sum: req.a + req.b }) })
        .build()
        .expect("router build")
}

fn bench_dispatch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Spin the server up once and reuse across every iteration.
    let (client, _server) = rt.block_on(async {
        let server = spawn_http(build_router());
        let url = server.server_url("/rpc").unwrap().to_string();
        let mut client = BenchSvcClientBuilder::new()
            .server_url(url)
            .build()
            .expect("client build");
        client.set_bearer_token(Some("user-token".to_string()));
        (client, server)
    });

    c.bench_function("jsonrpc_add_dispatch", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            async move {
                let r = client.add(AddRequest { a: 1, b: 2 }).await.expect("add ok");
                std::hint::black_box(r);
            }
        });
    });
}

criterion_group!(benches, bench_dispatch);
criterion_main!(benches);
