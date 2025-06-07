use anyhow::{Context, Result};
use axum::http::Method;
use axum::{
    Json, Router,
    extract::{Query, State},
    response::{Html, Redirect},
    routing::{get, post},
};
use rust_identity_core::{IdentityError, IdentityProvider};
use rust_identity_oauth2::{
    InMemoryStateStore, OAuth2AuthPayload, OAuth2Config, OAuth2Provider, OAuth2ProviderConfig,
    OAuth2Response,
};
use rust_identity_session::{JwtAuthProvider, SessionConfig, SessionService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

mod permissions;
mod service;

use permissions::GoogleOAuth2Permissions;

/// Configuration for the Google OAuth2 example
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub redirect_uri: String,
    pub jwt_secret: String,
    pub server_host: String,
    pub server_port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            google_client_id: std::env::var("GOOGLE_CLIENT_ID")
                .context("GOOGLE_CLIENT_ID environment variable is required")?,
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
                .context("GOOGLE_CLIENT_SECRET environment variable is required")?,
            redirect_uri: std::env::var("REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:3000/auth/callback".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production-please".to_string()),
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
        })
    }
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub session_service: Arc<SessionService>,
    pub oauth2_provider: Arc<OAuth2Provider>,
}

/// OAuth2 callback query parameters
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// OAuth2 flow initiation request
#[derive(Debug, Serialize, Deserialize)]
pub struct StartOAuth2Request {
    provider_id: String,
    additional_params: Option<HashMap<String, String>>,
}

/// OAuth2 flow initiation response
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StartOAuth2Response {
    authorization_url: String,
    state: String,
}

/// Session token response
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionTokenResponse {
    token: String,
    user_info: UserInfo,
}

/// User information response
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub subject: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub picture: Option<String>,
    pub permissions: Vec<String>,
}

impl Default for SessionTokenResponse {
    fn default() -> Self {
        Self {
            token: String::new(),
            user_info: UserInfo {
                subject: String::new(),
                email: None,
                display_name: None,
                picture: None,
                permissions: Vec::new(),
            },
        }
    }
}

/// Initialize the OAuth2 provider with Google configuration
fn create_oauth2_provider(config: &AppConfig) -> Result<OAuth2Provider> {
    let oauth2_config = OAuth2Config::new()
        .with_state_ttl(600) // 10 minutes
        .with_http_timeout(30); // 30 seconds

    let google_config = OAuth2ProviderConfig {
        provider_id: "google".to_string(),
        client_id: config.google_client_id.clone(),
        client_secret: config.google_client_secret.clone(),
        authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
        userinfo_endpoint: Some("https://www.googleapis.com/oauth2/v3/userinfo".to_string()),
        redirect_uri: config.redirect_uri.clone(),
        scopes: vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ],
        auth_params: {
            let mut params = HashMap::new();
            params.insert("access_type".to_string(), "offline".to_string());
            params.insert("prompt".to_string(), "consent".to_string());
            params
        },
        use_pkce: true,
        user_info_mapping: None,
    };

    let state_store = Arc::new(InMemoryStateStore::new());
    let mut provider = OAuth2Provider::new(oauth2_config, state_store);
    provider.add_provider(google_config);

    Ok(provider)
}

/// Initialize the session service
fn create_session_service(config: &AppConfig) -> Result<SessionService> {
    let session_config = SessionConfig {
        jwt_secret: config.jwt_secret.clone(),
        jwt_ttl: chrono::Duration::hours(24),
        refresh_enabled: true,
        algorithm: jsonwebtoken::Algorithm::HS256,
    };

    let permissions_provider = Arc::new(GoogleOAuth2Permissions::new());
    let session_service =
        SessionService::new(session_config).with_permissions(permissions_provider);

    Ok(session_service)
}

/// Handler for the root page - serves a simple HTML page with OAuth2 login
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

/// Handler to start the OAuth2 flow
async fn start_oauth2_handler(
    State(state): State<AppState>,
    Json(request): Json<StartOAuth2Request>,
) -> Result<Json<StartOAuth2Response>, String> {
    info!("Starting OAuth2 flow for provider: {}", request.provider_id);

    let auth_payload = OAuth2AuthPayload::StartFlow {
        provider_id: request.provider_id.clone(),
        additional_params: request.additional_params,
    };

    let payload_json = serde_json::to_value(auth_payload)
        .map_err(|e| format!("Failed to serialize OAuth2 payload: {}", e))?;

    // The OAuth2 provider returns an error with the authorization URL for start flow
    match state.oauth2_provider.verify(payload_json).await {
        Err(IdentityError::ProviderError(response_json)) => {
            let oauth2_response: OAuth2Response = serde_json::from_str(&response_json)
                .map_err(|e| format!("Failed to parse OAuth2 response: {}", e))?;

            match oauth2_response {
                OAuth2Response::AuthorizationUrl { url, state } => Ok(Json(StartOAuth2Response {
                    authorization_url: url,
                    state,
                })),
                OAuth2Response::Error { message } => Err(format!("OAuth2 error: {}", message)),
            }
        }
        Err(e) => Err(format!("OAuth2 provider error: {}", e)),
        Ok(_) => Err("Unexpected success response from start flow".to_string()),
    }
}

/// Handler for OAuth2 callback
async fn oauth2_callback_handler(
    State(state): State<AppState>,
    Query(callback_query): Query<CallbackQuery>,
) -> Result<Redirect, String> {
    info!("Handling OAuth2 callback");

    // Check for error in callback
    if let Some(error) = &callback_query.error {
        let error_desc = callback_query
            .error_description
            .as_deref()
            .unwrap_or("No description");
        error!("OAuth2 callback error: {}: {}", error, error_desc);
        return Ok(Redirect::to("/error"));
    }

    let code = callback_query
        .code
        .ok_or_else(|| "Missing authorization code in callback".to_string())?;

    let state_param = callback_query
        .state
        .ok_or_else(|| "Missing state parameter in callback".to_string())?;

    // Complete the OAuth2 flow
    let auth_payload = OAuth2AuthPayload::Callback {
        provider_id: "google".to_string(),
        code,
        state: state_param,
        error: callback_query.error,
        error_description: callback_query.error_description,
    };

    let payload_json = serde_json::to_value(auth_payload)
        .map_err(|e| format!("Failed to serialize callback payload: {}", e))?;

    // Create session using the session service
    let token = state
        .session_service
        .begin_session("oauth2", payload_json)
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

    info!("OAuth2 callback successful, redirecting with token");

    // Redirect to success page with token (in a real app, you'd handle this more securely)
    Ok(Redirect::to(&format!("/success?token={}", token)))
}

/// Handler for success page
async fn success_handler(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    let token = params.get("token").cloned().unwrap_or_default();
    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>OAuth2 Success</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .token {{ background: #f0f0f0; padding: 20px; margin: 20px 0; word-break: break-all; }}
        .button {{ background: #4285f4; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; }}
    </style>
</head>
<body>
    <h1>OAuth2 Authentication Successful!</h1>
    <p>You have successfully authenticated with Google OAuth2.</p>
    <p><strong>JWT Token:</strong></p>
    <div class="token">{}</div>
    <p>You can now use this token to make authenticated requests to the JSON-RPC API.</p>
    <a href="/" class="button">Back to Home</a>
    <a href="/api-docs" class="button">View API Documentation</a>
</body>
</html>
        "#,
        token
    );
    Html(html)
}

/// Handler for error page
async fn error_handler() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>OAuth2 Error</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .error { color: red; }
        .button { background: #4285f4; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; }
    </style>
</head>
<body>
    <h1 class="error">OAuth2 Authentication Failed</h1>
    <p>There was an error during the OAuth2 authentication process.</p>
    <a href="/" class="button">Try Again</a>
</body>
</html>
    "#,
    )
}

/// Handler for API documentation
async fn api_docs_handler() -> Html<&'static str> {
    Html(include_str!("../static/api-docs.html"))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables from .env file if present
    let _ = dotenvy::dotenv();

    // Load configuration
    let config = AppConfig::from_env()?;
    info!("Starting Google OAuth2 example server");

    // Create OAuth2 provider
    let oauth2_provider = Arc::new(create_oauth2_provider(&config)?);
    info!("OAuth2 provider initialized with Google configuration");

    // Create session service
    let session_service = Arc::new(create_session_service(&config)?);
    info!("Session service initialized");

    // Register OAuth2 provider with session service
    session_service
        .register_provider(Box::new((*oauth2_provider).clone()))
        .await;

    // Create application state
    let app_state = AppState {
        session_service: session_service.clone(),
        oauth2_provider,
    };

    // Create auth provider for JSON-RPC
    let auth_provider = JwtAuthProvider::new(session_service);

    // Build the application router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/auth/start", post(start_oauth2_handler))
        .route("/auth/callback", get(oauth2_callback_handler))
        .route("/success", get(success_handler))
        .route("/error", get(error_handler))
        .route("/api-docs", get(api_docs_handler))
        .nest_service(
            "/static",
            ServeDir::new("examples/google-oauth-example/static"),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any),
        )
        .with_state(app_state);

    // Add the JSON-RPC API routes to a separate router that will run on a different port
    // or integrate them directly into the main app
    let api_router = service::create_api_router(auth_provider);

    // For this example, let's run both on the same server by merging them manually
    // We'll create a new combined router
    let combined_app = Router::new()
        .merge(app)
        .merge(api_router.nest("/api", Router::new()));

    // Start the server
    let bind_addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {}", bind_addr))?;

    info!("Server running on http://{}", bind_addr);
    info!("OAuth2 redirect URI: {}", config.redirect_uri);
    warn!(
        "This is an example application. Do not use in production without proper security review."
    );

    axum::serve(listener, combined_app)
        .await
        .context("Server error")?;

    Ok(())
}
