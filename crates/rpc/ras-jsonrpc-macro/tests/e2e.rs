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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RenameUserV1 {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RenameUserV2 {
    display_name: String,
    notify: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RenameUserResponseV1 {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RenameUserResponseV2 {
    display_name: String,
    notified: bool,
}

struct RenameUserCompat;

impl ras_jsonrpc_core::VersionMigration<RenameUserV1, RenameUserV2> for RenameUserCompat {
    type Error = std::convert::Infallible;

    fn migrate(value: RenameUserV1) -> Result<RenameUserV2, Self::Error> {
        Ok(RenameUserV2 {
            display_name: value.name,
            notify: false,
        })
    }
}

impl ras_jsonrpc_core::VersionMigration<RenameUserResponseV2, RenameUserResponseV1>
    for RenameUserCompat
{
    type Error = std::convert::Infallible;

    fn migrate(value: RenameUserResponseV2) -> Result<RenameUserResponseV1, Self::Error> {
        Ok(RenameUserResponseV1 {
            name: value.display_name,
        })
    }
}

jsonrpc_service!({
    service_name: Demo,
    openrpc: false,
    methods: [
        UNAUTHORIZED ping(EchoRequest) -> EchoResponse,
        UNAUTHORIZED rename_user(RenameUserV2) -> RenameUserResponseV2 {
            version: v2,
            wire: "rename_user.v2",
            versions: [
                v1 {
                    wire: "rename_user.v1",
                    request: RenameUserV1,
                    response: RenameUserResponseV1,
                    migration: RenameUserCompat,
                },
            ],
        },
        WITH_PERMISSIONS(["user"]) add(AddRequest) -> AddResponse,
        WITH_PERMISSIONS(["admin"]) admin_only(EchoRequest) -> EchoResponse,
    ]
});

struct DemoImpl;

impl DemoTrait for DemoImpl {
    async fn ping(
        &self,
        req: EchoRequest,
    ) -> Result<EchoResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(EchoResponse {
            msg: req.msg,
            user_id: None,
        })
    }

    async fn add(
        &self,
        _user: &ras_jsonrpc_core::AuthenticatedUser,
        req: AddRequest,
    ) -> Result<AddResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(AddResponse { sum: req.a + req.b })
    }

    async fn rename_user(
        &self,
        req: RenameUserV2,
    ) -> Result<RenameUserResponseV2, Box<dyn std::error::Error + Send + Sync>> {
        Ok(RenameUserResponseV2 {
            display_name: req.display_name,
            notified: req.notify,
        })
    }

    async fn admin_only(
        &self,
        user: &ras_jsonrpc_core::AuthenticatedUser,
        req: EchoRequest,
    ) -> Result<EchoResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(EchoResponse {
            msg: req.msg,
            user_id: Some(user.user_id.clone()),
        })
    }
}

fn router() -> axum::Router {
    DemoBuilder::new(DemoImpl)
        .base_url("/rpc")
        .auth_provider(MockAuthProvider::default())
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
async fn legacy_version_round_trips_through_canonical_handler() {
    let server = spawn_http(router());
    let url = server.server_url("/rpc").expect("server url").to_string();

    let resp = client(url)
        .rename_user_v1(RenameUserV1 {
            name: "Ada".to_string(),
        })
        .await
        .expect("legacy rename ok");

    assert_eq!(
        resp,
        RenameUserResponseV1 {
            name: "Ada".to_string()
        }
    );
}

#[tokio::test]
async fn canonical_version_uses_declared_wire_method() {
    let server = spawn_http(router());
    let url = server.server_url("/rpc").expect("server url").to_string();

    let resp = client(url)
        .rename_user(RenameUserV2 {
            display_name: "Grace".to_string(),
            notify: true,
        })
        .await
        .expect("canonical rename ok");

    assert_eq!(
        resp,
        RenameUserResponseV2 {
            display_name: "Grace".to_string(),
            notified: true,
        }
    );
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
