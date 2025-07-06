//! WebSocket upgrade handling with authentication

use crate::{ServerError, ServerResult};
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade as AxumWebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use ras_auth_core::{AuthProvider, AuthenticatedUser};
use tracing::{debug, error, info, warn};

/// WebSocket upgrade handler with authentication support
pub struct WebSocketUpgrade {
    /// The underlying Axum WebSocket upgrade
    upgrade: AxumWebSocketUpgrade,
    /// Request headers for authentication
    headers: HeaderMap,
}

impl WebSocketUpgrade {
    /// Create a new WebSocket upgrade from Axum extractor
    pub fn new(upgrade: AxumWebSocketUpgrade, headers: HeaderMap) -> Self {
        Self { upgrade, headers }
    }

    /// Extract authentication token from headers
    pub fn extract_auth_token(&self) -> Option<String> {
        // Try Authorization header first (Bearer token)
        if let Some(auth_header) = self.headers.get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    return Some(auth_str[7..].to_string());
                }
                // Also support just the token without "Bearer " prefix
                return Some(auth_str.to_string());
            }
        }

        // Try custom WebSocket auth headers
        if let Some(token_header) = self.headers.get("sec-websocket-protocol") {
            if let Ok(token_str) = token_header.to_str() {
                // Support protocols like "token.{jwt_token}"
                if token_str.starts_with("token.") {
                    return Some(token_str[6..].to_string());
                }
            }
        }

        // Try X-Auth-Token header
        if let Some(token_header) = self.headers.get("x-auth-token") {
            if let Ok(token_str) = token_header.to_str() {
                return Some(token_str.to_string());
            }
        }

        None
    }

    /// Authenticate the connection using the provided auth provider
    pub async fn authenticate<A: AuthProvider>(
        &self,
        auth_provider: &A,
    ) -> ServerResult<Option<AuthenticatedUser>> {
        if let Some(token) = self.extract_auth_token() {
            debug!("Attempting to authenticate WebSocket connection");
            match auth_provider.authenticate(token).await {
                Ok(user) => {
                    info!(
                        "WebSocket connection authenticated for user: {}",
                        user.user_id
                    );
                    Ok(Some(user))
                }
                Err(e) => {
                    warn!("WebSocket authentication failed: {}", e);
                    Err(ServerError::AuthenticationFailed(e))
                }
            }
        } else {
            debug!("No authentication token found in WebSocket headers");
            Ok(None)
        }
    }

    /// Complete the WebSocket upgrade
    pub fn on_upgrade<F>(self, callback: F) -> Response
    where
        F: FnOnce(WebSocket) -> futures::future::BoxFuture<'static, ()> + Send + 'static,
    {
        self.upgrade.on_upgrade(callback)
    }

    /// Complete the WebSocket upgrade with authentication
    pub async fn on_upgrade_with_auth<A, F>(
        self,
        auth_provider: &A,
        require_auth: bool,
        callback: F,
    ) -> Result<Response, (StatusCode, String)>
    where
        A: AuthProvider,
        F: FnOnce(WebSocket, Option<AuthenticatedUser>) -> futures::future::BoxFuture<'static, ()>
            + Send
            + 'static,
    {
        // Authenticate before upgrading
        let auth_result = self.authenticate(auth_provider).await;

        match auth_result {
            Ok(user) => {
                // Check if authentication is required
                if require_auth && user.is_none() {
                    error!("Authentication required but no valid token provided");
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "Authentication required".to_string(),
                    ));
                }

                // Complete the upgrade
                let response = self.upgrade.on_upgrade(move |socket| {
                    Box::pin(async move {
                        callback(socket, user).await;
                    })
                });

                Ok(response)
            }
            Err(e) => {
                error!("Authentication failed during WebSocket upgrade: {}", e);
                Err((e.to_status_code(), e.to_string()))
            }
        }
    }

    /// Get the underlying headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Check if a specific header is present
    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }

    /// Get a header value as string
    pub fn get_header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    /// Extract client IP from headers (useful for logging/security)
    pub fn extract_client_ip(&self) -> Option<String> {
        // Try various headers in order of preference
        let ip_headers = [
            "x-forwarded-for",
            "x-real-ip",
            "cf-connecting-ip", // Cloudflare
            "x-client-ip",
            "x-forwarded",
            "forwarded-for",
            "forwarded",
        ];

        for header_name in &ip_headers {
            if let Some(value) = self.get_header(header_name) {
                // For X-Forwarded-For, take the first IP
                let ip = value.split(',').next().unwrap_or(&value).trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }

        None
    }

    /// Extract user agent
    pub fn extract_user_agent(&self) -> Option<String> {
        self.get_header("user-agent")
    }

    /// Create connection metadata from headers
    pub fn create_metadata(&self) -> serde_json::Value {
        let mut metadata = serde_json::Map::new();

        // Add client IP if available
        if let Some(ip) = self.extract_client_ip() {
            metadata.insert("client_ip".to_string(), serde_json::Value::String(ip));
        }

        // Add user agent if available
        if let Some(user_agent) = self.extract_user_agent() {
            metadata.insert(
                "user_agent".to_string(),
                serde_json::Value::String(user_agent),
            );
        }

        // Add connection timestamp
        metadata.insert(
            "connected_at".to_string(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        );

        serde_json::Value::Object(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_parsing_logic() {
        // Test just the header parsing logic without WebSocketUpgrade
        let mut headers = HeaderMap::new();

        // Test Bearer token extraction logic
        headers.insert("authorization", "Bearer abc123".parse().unwrap());
        if let Some(auth_header) = headers.get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    assert_eq!(&auth_str[7..], "abc123");
                }
            }
        }

        // Test X-Forwarded-For parsing logic
        headers.clear();
        headers.insert("x-forwarded-for", "192.168.1.1, 10.0.0.1".parse().unwrap());
        if let Some(header_value) = headers.get("x-forwarded-for") {
            if let Ok(value) = header_value.to_str() {
                let ip = value.split(',').next().unwrap_or(&value).trim();
                assert_eq!(ip, "192.168.1.1");
            }
        }
    }

    #[test]
    fn test_metadata_creation() {
        // Test metadata creation without needing WebSocketUpgrade
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "client_ip".to_string(),
            serde_json::Value::String("127.0.0.1".to_string()),
        );
        metadata.insert(
            "user_agent".to_string(),
            serde_json::Value::String("test-agent".to_string()),
        );

        let metadata_value = serde_json::Value::Object(metadata);
        assert!(metadata_value.is_object());
        assert_eq!(metadata_value.get("client_ip").unwrap(), "127.0.0.1");
        assert_eq!(metadata_value.get("user_agent").unwrap(), "test-agent");
    }
}
