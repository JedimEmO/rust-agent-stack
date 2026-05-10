use std::collections::HashSet;

use anyhow::Result;
use axum::Router;
use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_jsonrpc_macro::jsonrpc_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request payload for the `ping` method.
///
/// **Schema docs** should render with Markdown.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PingRequest {
    /// Message echoed by the fixture service.
    /// This line must stay on a new line.
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RenameWidgetV1 {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RenameWidgetV2 {
    pub display_name: String,
    pub notify: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RenameWidgetResponseV1 {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RenameWidgetResponseV2 {
    pub display_name: String,
    pub notified: bool,
}

jsonrpc_service!({
    service_name: ExplorerRpcFixture,
    openrpc: true,
    explorer: true,
    methods: [
        /// Echo a `PingRequest` message.
        ///
        /// **Use this in tests.**
        /// - Confirms list rendering
        /// - Preserves list items
        ///
        /// Line one
        /// Line two
        ///
        /// ```json
        /// {"message":"hello"}
        /// ```
        ///
        /// See [Rust API Stack](https://example.com/docs).
        UNAUTHORIZED ping(PingRequest) -> PingResponse,
        UNAUTHORIZED no_params(()) -> String,
        UNAUTHORIZED rename_widget(RenameWidgetV2) -> RenameWidgetResponseV2 {
            version: v2,
            wire: "rename_widget.v2",
            versions: [
                v1 {
                    wire: "rename_widget.v1",
                    request: RenameWidgetV1,
                    response: RenameWidgetResponseV1,
                    migration: RenameWidgetCompat,
                },
            ],
        },
        WITH_PERMISSIONS(["admin"]) create_widget(CreateWidgetRequest) -> Widget,
        WITH_PERMISSIONS(["user"]) current_profile(()) -> ProfileResponse,
    ]
});

struct RenameWidgetCompat;

impl ras_jsonrpc_core::VersionMigration<RenameWidgetV1, RenameWidgetV2> for RenameWidgetCompat {
    type Error = std::convert::Infallible;

    fn migrate(value: RenameWidgetV1) -> Result<RenameWidgetV2, Self::Error> {
        Ok(RenameWidgetV2 {
            display_name: value.name,
            notify: false,
        })
    }
}

impl ras_jsonrpc_core::VersionMigration<RenameWidgetResponseV2, RenameWidgetResponseV1>
    for RenameWidgetCompat
{
    type Error = std::convert::Infallible;

    fn migrate(value: RenameWidgetResponseV2) -> Result<RenameWidgetResponseV1, Self::Error> {
        Ok(RenameWidgetResponseV1 {
            name: value.display_name,
        })
    }
}

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

struct ExplorerRpcFixtureImpl;

impl ExplorerRpcFixtureTrait for ExplorerRpcFixtureImpl {
    async fn ping(
        &self,
        request: PingRequest,
    ) -> std::result::Result<PingResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(PingResponse {
            message: format!("pong: {}", request.message),
        })
    }

    async fn no_params(
        &self,
        _request: (),
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok("no params ok".to_string())
    }

    async fn rename_widget(
        &self,
        request: RenameWidgetV2,
    ) -> std::result::Result<RenameWidgetResponseV2, Box<dyn std::error::Error + Send + Sync>> {
        Ok(RenameWidgetResponseV2 {
            display_name: request.display_name,
            notified: request.notify,
        })
    }

    async fn create_widget(
        &self,
        _user: &AuthenticatedUser,
        request: CreateWidgetRequest,
    ) -> std::result::Result<Widget, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Widget {
            id: "rpc-created-widget".to_string(),
            name: request.name,
            owner: request.owner,
        })
    }

    async fn current_profile(
        &self,
        user: &AuthenticatedUser,
        _request: (),
    ) -> std::result::Result<ProfileResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ProfileResponse {
            user_id: user.user_id.clone(),
            permissions: user.permissions.iter().cloned().collect(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_router = ExplorerRpcFixtureBuilder::new(ExplorerRpcFixtureImpl)
        .base_url("/rpc")
        .auth_provider(FixtureAuthProvider)
        .build()
        .expect("fixture JSON-RPC service should build");

    let app = Router::new().merge(rpc_router);
    let bind_addr =
        std::env::var("PLAYWRIGHT_JSONRPC_ADDR").unwrap_or_else(|_| "127.0.0.1:3102".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
