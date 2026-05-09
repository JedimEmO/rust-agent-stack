use std::collections::HashSet;

use anyhow::Result;
use axum::Router;
use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_jsonrpc_macro::jsonrpc_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request payload for the ping method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PingRequest {
    /// Message echoed by the fixture service.
    pub message: String,
}

/// Response returned by the ping method.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PingResponse {
    /// Message returned from the fixture service.
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateWidgetRequest {
    pub name: String,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Widget {
    pub id: String,
    pub name: String,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfileResponse {
    pub user_id: String,
    pub permissions: Vec<String>,
}

jsonrpc_service!({
    service_name: ExplorerRpcFixture,
    openrpc: true,
    explorer: true,
    methods: [
        /// Echo a ping message.
        ///
        /// Used by explorer tests to verify OpenRPC method docs render.
        UNAUTHORIZED ping(PingRequest) -> PingResponse,
        UNAUTHORIZED no_params(()) -> String,
        WITH_PERMISSIONS(["admin"]) create_widget(CreateWidgetRequest) -> Widget,
        WITH_PERMISSIONS(["user"]) current_profile(()) -> ProfileResponse,
    ]
});

struct FixtureAuthProvider;

impl AuthProvider for FixtureAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            let (user_id, permissions) = match token.as_str() {
                "user-token" => ("user-1", vec!["user"]),
                "admin-token" => ("admin-1", vec!["user", "admin"]),
                _ => return Err(AuthError::InvalidToken),
            };

            Ok(AuthenticatedUser {
                user_id: user_id.to_string(),
                permissions: permissions
                    .into_iter()
                    .map(str::to_string)
                    .collect::<HashSet<_>>(),
                metadata: None,
            })
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_router = ExplorerRpcFixtureBuilder::new("/rpc")
        .auth_provider(FixtureAuthProvider)
        .ping_handler(|request| async move {
            Ok(PingResponse {
                message: format!("pong: {}", request.message),
            })
        })
        .no_params_handler(|_request| async move { Ok("no params ok".to_string()) })
        .create_widget_handler(|_user, request| async move {
            Ok(Widget {
                id: "rpc-created-widget".to_string(),
                name: request.name,
                owner: request.owner,
            })
        })
        .current_profile_handler(|user, _request| async move {
            Ok(ProfileResponse {
                user_id: user.user_id.clone(),
                permissions: user.permissions.iter().cloned().collect(),
            })
        })
        .build()
        .expect("fixture JSON-RPC service should build");

    let app = Router::new().merge(rpc_router);
    let bind_addr =
        std::env::var("PLAYWRIGHT_JSONRPC_ADDR").unwrap_or_else(|_| "127.0.0.1:3102".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
