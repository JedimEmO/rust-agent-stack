use std::collections::HashSet;

use anyhow::Result;
use ras_auth_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_rest_core::{RestResponse, RestResult};
use ras_rest_macro::rest_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Widget {
    pub id: String,
    pub name: String,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WidgetsResponse {
    pub widgets: Vec<Widget>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateWidgetRequest {
    pub name: String,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfileResponse {
    pub user_id: String,
    pub permissions: Vec<String>,
}

rest_service!({
    service_name: ExplorerRestFixture,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,
    docs_path: "/docs",
    endpoints: [
        GET UNAUTHORIZED health() -> HealthResponse,
        GET UNAUTHORIZED widgets/{id: String}() -> Widget,
        GET UNAUTHORIZED search/widgets ? q: String & limit: Option<u32> () -> WidgetsResponse,
        POST WITH_PERMISSIONS(["admin"]) widgets(CreateWidgetRequest) -> Widget,
        GET WITH_PERMISSIONS(["user"]) profile() -> ProfileResponse,
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

struct FixtureService;

#[async_trait::async_trait]
impl ExplorerRestFixtureTrait for FixtureService {
    async fn get_health(&self) -> RestResult<HealthResponse> {
        Ok(RestResponse::ok(HealthResponse {
            status: "ok".to_string(),
        }))
    }

    async fn get_widgets_by_id(&self, id: String) -> RestResult<Widget> {
        Ok(RestResponse::ok(Widget {
            id,
            name: "Fixture Widget".to_string(),
            owner: "public".to_string(),
        }))
    }

    async fn get_search_widgets(
        &self,
        q: String,
        limit: Option<u32>,
    ) -> RestResult<WidgetsResponse> {
        let count = limit.unwrap_or(2).min(5) as usize;
        let widgets = (0..count)
            .map(|idx| Widget {
                id: format!("widget-{idx}"),
                name: format!("{q}-{idx}"),
                owner: "search".to_string(),
            })
            .collect::<Vec<_>>();

        Ok(RestResponse::ok(WidgetsResponse {
            total: widgets.len(),
            widgets,
        }))
    }

    async fn post_widgets(
        &self,
        _user: &AuthenticatedUser,
        request: CreateWidgetRequest,
    ) -> RestResult<Widget> {
        Ok(RestResponse::created(Widget {
            id: "created-widget".to_string(),
            name: request.name,
            owner: request.owner,
        }))
    }

    async fn get_profile(&self, user: &AuthenticatedUser) -> RestResult<ProfileResponse> {
        Ok(RestResponse::ok(ProfileResponse {
            user_id: user.user_id.clone(),
            permissions: user.permissions.iter().cloned().collect(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = ExplorerRestFixtureBuilder::new(FixtureService)
        .auth_provider(FixtureAuthProvider)
        .build();

    let bind_addr =
        std::env::var("PLAYWRIGHT_REST_ADDR").unwrap_or_else(|_| "127.0.0.1:3101".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
