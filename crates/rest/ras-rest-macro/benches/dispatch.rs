//! Criterion bench measuring per-call latency of an authenticated REST GET
//! through the full stack: generated client → axum router → handler.
//!
//! Run with `cargo bench -p ras-rest-macro`.

use criterion::{Criterion, criterion_group, criterion_main};
use ras_auth_core::AuthenticatedUser;
use ras_rest_core::{RestResponse, RestResult};
use ras_rest_macro::rest_service;
use ras_test_helpers::{MockAuthProvider, spawn_http};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct Item {
    id: u32,
    name: String,
}

rest_service!({
    service_name: BenchSvc,
    base_path: "/api",
    openapi: false,
    serve_docs: false,
    endpoints: [
        GET WITH_PERMISSIONS(["user"]) items/{id: u32}() -> Item,
    ]
});

struct BenchImpl;

#[async_trait::async_trait]
impl BenchSvcTrait for BenchImpl {
    async fn get_items_by_id(&self, _user: &AuthenticatedUser, id: u32) -> RestResult<Item> {
        Ok(RestResponse::ok(Item {
            id,
            name: "x".into(),
        }))
    }
}

fn build_router() -> axum::Router {
    BenchSvcBuilder::new(BenchImpl)
        .auth_provider(MockAuthProvider::default())
        .build()
}

fn bench_dispatch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (client, _server) = rt.block_on(async {
        let server = spawn_http(build_router());
        let base = server.server_address().unwrap().to_string();
        let mut client = BenchSvcClient::builder(&base)
            .build()
            .expect("client build");
        client.set_bearer_token(Some("user-token".to_string()));
        (client, server)
    });

    c.bench_function("rest_get_dispatch", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            async move {
                let r = client.get_items_by_id(1).await.expect("get ok");
                std::hint::black_box(r);
            }
        });
    });
}

criterion_group!(benches, bench_dispatch);
criterion_main!(benches);
