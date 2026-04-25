//! Criterion bench measuring c2s call round-trip latency through a real
//! WebSocket connection (tokio-tungstenite client → axum server).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::{Router, routing::get};
use criterion::{Criterion, criterion_group, criterion_main};
use ras_auth_core::AuthenticatedUser;
use ras_jsonrpc_bidirectional_macro::jsonrpc_bidirectional_service;
use ras_jsonrpc_bidirectional_server::DefaultConnectionManager;
use ras_jsonrpc_bidirectional_server::service::{BuiltWebSocketService, websocket_handler};
use ras_jsonrpc_bidirectional_types::ConnectionId;
use ras_test_helpers::{MockAuthProvider, spawn_tcp};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EchoIn {
    msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EchoOut {
    msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ignored;

jsonrpc_bidirectional_service!({
    service_name: BenchSvc,
    client_to_server: [
        WITH_PERMISSIONS(["user"]) echo(EchoIn) -> EchoOut,
    ],
    server_to_client: [
        unused(Ignored),
    ],
    server_to_client_calls: [
    ]
});

#[derive(Clone)]
struct BenchImpl;

#[async_trait]
impl BenchSvcService for BenchImpl {
    async fn echo(
        &self,
        _client: ConnectionId,
        _conns: &dyn ras_jsonrpc_bidirectional_types::ConnectionManager,
        _user: &AuthenticatedUser,
        req: EchoIn,
    ) -> Result<EchoOut, Box<dyn std::error::Error + Send + Sync>> {
        Ok(EchoOut { msg: req.msg })
    }

    async fn notify_unused(
        &self,
        _connection_id: ConnectionId,
        _params: Ignored,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }
}

async fn start_server() -> String {
    let cm = Arc::new(DefaultConnectionManager::new());
    let handler = Arc::new(BenchSvcHandler::new(Arc::new(BenchImpl), cm.clone()));
    let svc = ras_jsonrpc_bidirectional_server::WebSocketServiceBuilder::builder()
        .handler(handler)
        .auth_provider(Arc::new(MockAuthProvider::default()))
        .require_auth(false)
        .build()
        .build_with_manager(cm);

    type SvcType = BuiltWebSocketService<
        BenchSvcHandler<BenchImpl, DefaultConnectionManager>,
        MockAuthProvider,
        DefaultConnectionManager,
    >;
    let app: Router = Router::new()
        .route("/ws", get(websocket_handler::<SvcType>))
        .with_state(svc);
    let (addr, _h) = spawn_tcp(app).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    format!("ws://{addr}/ws")
}

fn bench_roundtrip(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let client = rt.block_on(async {
        let url = start_server().await;
        let client = BenchSvcClientBuilder::new(url)
            .with_jwt_token("user-token".to_string())
            .build()
            .await
            .expect("client build");
        client.connect().await.expect("connect");
        client
    });

    c.bench_function("ws_echo_roundtrip", |b| {
        b.to_async(&rt).iter(|| async {
            let r = client.echo(EchoIn { msg: "x".into() }).await.expect("echo");
            std::hint::black_box(r);
        });
    });

    rt.block_on(async {
        let _ = client.disconnect().await;
    });
}

criterion_group!(benches, bench_roundtrip);
criterion_main!(benches);
