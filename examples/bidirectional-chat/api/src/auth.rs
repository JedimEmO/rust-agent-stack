//! REST API definitions for authentication endpoints

use ras_rest_macro::rest_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request payload for user login
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginRequest {
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
    /// Optional provider ID (defaults to "local")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Request payload for user registration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegisterRequest {
    /// Username for the new account
    pub username: String,
    /// Password for the new account
    pub password: String,
    /// Optional email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Optional display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Response payload for successful authentication
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoginResponse {
    /// JWT token for authentication
    pub token: String,
    /// Token expiration timestamp (Unix timestamp)
    pub expires_at: i64,
    /// User ID
    pub user_id: String,
}

/// Response payload for successful registration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegisterResponse {
    /// Success message
    pub message: String,
    /// Username of the created user
    pub username: String,
    /// Display name if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Response payload for health check
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HealthResponse {
    /// Health status
    pub status: String,
    /// Server timestamp
    pub timestamp: String,
}

// Define the REST service
rest_service!({
    service_name: ChatAuthService,
    base_path: "/",
    openapi: true,
    serve_docs: false,
    endpoints: [
        // Authentication endpoints
        POST UNAUTHORIZED auth/login(LoginRequest) -> LoginResponse,
        POST UNAUTHORIZED auth/register(RegisterRequest) -> RegisterResponse,

        // Health check endpoint
        GET UNAUTHORIZED health() -> HealthResponse,
    ]
});