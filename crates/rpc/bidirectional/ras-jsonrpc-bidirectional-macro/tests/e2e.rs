//! End-to-end test for `jsonrpc_bidirectional_service!`:
//!   generated client → real WebSocket → server handler → response/notification.
//!
//! Existing `bidirectional_integration.rs` exercises this thoroughly. This file
//! is a slim companion test that uses the shared `MockAuthProvider` and proves
//! the helper integration works.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use axum::{Router, routing::get};
use ras_auth_core::AuthenticatedUser;
use ras_jsonrpc_bidirectional_macro::jsonrpc_bidirectional_service;
use ras_jsonrpc_bidirectional_server::DefaultConnectionManager;
use ras_jsonrpc_bidirectional_server::service::{BuiltWebSocketService, websocket_handler};
use ras_jsonrpc_bidirectional_types::ConnectionId;
use ras_test_helpers::{MockAuthProvider, spawn_tcp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoIn {
    pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoOut {
    pub msg: String,
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNote {
    pub kind: String,
}

jsonrpc_bidirectional_service!({
    service_name: Demo,
    client_to_server: [
        UNAUTHORIZED hello(String) -> String,
        WITH_PERMISSIONS(["user"]) echo(EchoIn) -> EchoOut,
    ],
    server_to_client: [
        ping(PushNote),
    ],
    server_to_client_calls: [
    ]
});

#[derive(Clone)]
struct DemoImpl;

#[async_trait]
impl DemoService for DemoImpl {
    async fn hello(
        &self,
        _client: ConnectionId,
        _conns: &dyn ras_jsonrpc_bidirectional_types::ConnectionManager,
        name: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(format!("hello, {name}"))
    }

    async fn echo(
        &self,
        client: ConnectionId,
        conns: &dyn ras_jsonrpc_bidirectional_types::ConnectionManager,
        user: &AuthenticatedUser,
        req: EchoIn,
    ) -> Result<EchoOut, Box<dyn std::error::Error + Send + Sync>> {
        // Also push a server→client notification so the test can observe it.
        let note = ras_jsonrpc_bidirectional_types::ServerNotification {
            method: "ping".to_string(),
            params: serde_json::to_value(PushNote {
                kind: "after-echo".into(),
            })
            .unwrap(),
            metadata: None,
        };
        let _ = conns
            .send_to_connection(
                client,
                ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(note),
            )
            .await;

        Ok(EchoOut {
            msg: req.msg,
            user: user.user_id.clone(),
        })
    }

    async fn notify_ping(
        &self,
        _connection_id: ConnectionId,
        _params: PushNote,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }
}

async fn start_server() -> String {
    let connection_manager = Arc::new(DefaultConnectionManager::new());
    let handler = Arc::new(DemoHandler::new(
        Arc::new(DemoImpl),
        connection_manager.clone(),
    ));

    let ws_service = ras_jsonrpc_bidirectional_server::WebSocketServiceBuilder::builder()
        .handler(handler)
        .auth_provider(Arc::new(MockAuthProvider::default()))
        .require_auth(false)
        .build()
        .build_with_manager(connection_manager);

    type SvcType = BuiltWebSocketService<
        DemoHandler<DemoImpl, DefaultConnectionManager>,
        MockAuthProvider,
        DefaultConnectionManager,
    >;
    let app: Router = Router::new()
        .route("/ws", get(websocket_handler::<SvcType>))
        .with_state(ws_service);

    let (addr, _handle) = spawn_tcp(app).await;
    // Give axum a tick to start serving.
    tokio::time::sleep(Duration::from_millis(50)).await;
    format!("ws://{addr}/ws")
}

#[tokio::test]
async fn unauthorized_method_round_trips() {
    let url = start_server().await;
    let client = DemoClientBuilder::new(url)
        .build()
        .await
        .expect("client build");
    client.connect().await.expect("connect");

    let resp = client.hello("alice".to_string()).await.expect("hello ok");
    assert_eq!(resp, "hello, alice");

    client.disconnect().await.expect("disconnect");
}

#[tokio::test]
async fn auth_method_succeeds_and_pushes_notification() {
    let url = start_server().await;
    let mut client = DemoClientBuilder::new(url)
        .with_jwt_token("user-token".to_string())
        .build()
        .await
        .expect("client build");
    client.connect().await.expect("connect");

    let pushed = Arc::new(AtomicBool::new(false));
    let pushed_flag = pushed.clone();
    client.on_ping(move |_n: PushNote| {
        pushed_flag.store(true, Ordering::SeqCst);
    });

    let resp = client
        .echo(EchoIn {
            msg: "hi".to_string(),
        })
        .await
        .expect("echo ok");
    assert_eq!(resp.msg, "hi");
    assert_eq!(resp.user, "user-1");

    // Wait briefly for the push to land.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !pushed.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        pushed.load(Ordering::SeqCst),
        "expected ping notification to arrive"
    );

    client.disconnect().await.expect("disconnect");
}

#[tokio::test]
async fn auth_method_rejected_for_readonly_user() {
    let url = start_server().await;
    let client = DemoClientBuilder::new(url)
        .with_jwt_token("readonly-token".to_string())
        .build()
        .await
        .expect("client build");
    client.connect().await.expect("connect");

    let result = client.echo(EchoIn { msg: "nope".into() }).await;
    assert!(
        result.is_err(),
        "readonly token must not be able to call echo"
    );

    client.disconnect().await.expect("disconnect");
}
