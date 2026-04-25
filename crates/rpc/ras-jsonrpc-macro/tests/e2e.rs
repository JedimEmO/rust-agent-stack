//! End-to-end test that exercises the full chain:
//!   generated reqwest client → axum router → handler → response → client.
//!
//! Covers: success path, missing-permission rejection, malformed input.

use ras_jsonrpc_macro::jsonrpc_service;
use ras_test_helpers::{MockAuthProvider, spawn_http};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct EchoRequest {
    msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct EchoResponse {
    msg: String,
    user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AddRequest {
    a: i64,
    b: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AddResponse {
    sum: i64,
}

jsonrpc_service!({
    service_name: Demo,
    openrpc: false,
    methods: [
        UNAUTHORIZED ping(EchoRequest) -> EchoResponse,
        WITH_PERMISSIONS(["user"]) add(AddRequest) -> AddResponse,
        WITH_PERMISSIONS(["admin"]) admin_only(EchoRequest) -> EchoResponse,
    ]
});

fn router() -> axum::Router {
    DemoBuilder::new("/rpc")
        .auth_provider(MockAuthProvider::default())
        .ping_handler(|req: EchoRequest| async move {
            Ok(EchoResponse {
                msg: req.msg,
                user_id: None,
            })
        })
        .add_handler(|_user, req: AddRequest| async move { Ok(AddResponse { sum: req.a + req.b }) })
        .admin_only_handler(|user, req: EchoRequest| async move {
            Ok(EchoResponse {
                msg: req.msg,
                user_id: Some(user.user_id),
            })
        })
        .build()
        .expect("build router")
}

fn client(url: String) -> DemoClient {
    DemoClientBuilder::new()
        .server_url(url)
        .build()
        .expect("client build")
}

#[tokio::test]
async fn unauth_method_round_trips() {
    let server = spawn_http(router());
    let url = server.server_url("/rpc").expect("server url").to_string();

    let mut c = client(url);
    c.set_bearer_token(Option::<String>::None);

    let resp = c
        .ping(EchoRequest {
            msg: "hello".to_string(),
        })
        .await
        .expect("ping ok");

    assert_eq!(resp.msg, "hello");
    assert_eq!(resp.user_id, None);
}

#[tokio::test]
async fn permission_required_method_rejects_anonymous() {
    let server = spawn_http(router());
    let url = server.server_url("/rpc").unwrap().to_string();

    let mut c = client(url);
    c.set_bearer_token(Option::<String>::None);

    let err = c
        .add(AddRequest { a: 2, b: 3 })
        .await
        .expect_err("anonymous add must be rejected");

    let s = err.to_string();
    assert!(
        s.contains("Authentication") || s.contains("AUTH") || s.contains("auth"),
        "expected auth-related error, got: {s}"
    );
}

#[tokio::test]
async fn permission_required_method_rejects_wrong_perms() {
    let server = spawn_http(router());
    let url = server.server_url("/rpc").unwrap().to_string();

    let mut c = client(url);
    c.set_bearer_token(Some("readonly-token".to_string()));

    let err = c
        .add(AddRequest { a: 2, b: 3 })
        .await
        .expect_err("readonly user must not be allowed to call add");
    let s = err.to_string();
    assert!(
        s.contains("permission") || s.contains("Permission") || s.contains("PERMISSION"),
        "expected permission-related error, got: {s}"
    );
}

#[tokio::test]
async fn permission_required_method_succeeds_with_correct_perms() {
    let server = spawn_http(router());
    let url = server.server_url("/rpc").unwrap().to_string();

    let mut c = client(url);
    c.set_bearer_token(Some("user-token".to_string()));

    let resp = c.add(AddRequest { a: 7, b: 35 }).await.expect("add ok");
    assert_eq!(resp.sum, 42);
}

#[tokio::test]
async fn admin_method_succeeds_with_admin_token() {
    let server = spawn_http(router());
    let url = server.server_url("/rpc").unwrap().to_string();

    let mut c = client(url);
    c.set_bearer_token(Some("admin-token".to_string()));

    let resp = c
        .admin_only(EchoRequest {
            msg: "secret".to_string(),
        })
        .await
        .expect("admin call ok");

    assert_eq!(resp.msg, "secret");
    assert_eq!(resp.user_id.as_deref(), Some("admin-1"));
}

#[tokio::test]
async fn malformed_params_yield_jsonrpc_error() {
    // Bypass the typed client to send a malformed body and confirm the
    // server returns a JSON-RPC `invalid_params` error rather than a panic.
    let server = spawn_http(router());
    let url = server.server_url("/rpc").unwrap().to_string();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "ping",
        "params": { "bogus": 1 },
        "id": 1,
    });

    let resp: serde_json::Value = reqwest::Client::new()
        .post(url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(
        resp.get("error").is_some(),
        "expected error in response: {resp}"
    );
    let code = resp["error"]["code"].as_i64().unwrap();
    assert_eq!(code, -32602, "expected invalid_params (-32602), got {code}");
}
