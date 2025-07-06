//! Error types for WebSocket server operations

use axum::http::StatusCode;
use ras_auth_core::AuthError;
use ras_jsonrpc_bidirectional_types::BidirectionalError;
use thiserror::Error;

/// Server-specific errors for WebSocket operations
#[derive(Debug, Error)]
pub enum ServerError {
    /// WebSocket upgrade failed
    #[error("WebSocket upgrade failed: {0}")]
    UpgradeFailed(String),

    /// Authentication failed during connection
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(#[from] AuthError),

    /// Connection not found
    #[error("Connection {0} not found")]
    ConnectionNotFound(String),

    /// Message routing failed
    #[error("Message routing failed: {0}")]
    RoutingFailed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// WebSocket protocol error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Connection management error
    #[error("Connection management error: {0}")]
    ConnectionError(#[from] BidirectionalError),

    /// Internal server error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Invalid request format
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Handler not found for method
    #[error("No handler found for method: {0}")]
    HandlerNotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

impl ServerError {
    /// Convert to HTTP status code for upgrade errors
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            ServerError::AuthenticationFailed(_) => StatusCode::UNAUTHORIZED,
            ServerError::PermissionDenied(_) => StatusCode::FORBIDDEN,
            ServerError::ConnectionNotFound(_) => StatusCode::NOT_FOUND,
            ServerError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ServerError::HandlerNotFound(_) => StatusCode::NOT_IMPLEMENTED,
            ServerError::UpgradeFailed(_)
            | ServerError::RoutingFailed(_)
            | ServerError::SerializationError(_)
            | ServerError::WebSocketError(_)
            | ServerError::ConnectionError(_)
            | ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Convenience type alias for server operation results
pub type ServerResult<T> = Result<T, ServerError>;
