//! Error types for bidirectional JSON-RPC communication

use thiserror::Error;

/// Errors that can occur in bidirectional JSON-RPC communication
#[derive(Error, Debug)]
pub enum BidirectionalError {
    /// Connection not found
    #[error("Connection not found: {0}")]
    ConnectionNotFound(crate::ConnectionId),

    /// Connection already exists
    #[error("Connection already exists: {0}")]
    ConnectionAlreadyExists(crate::ConnectionId),

    /// Failed to send message
    #[error("Failed to send message: {0}")]
    SendError(String),

    /// Failed to broadcast message
    #[error("Failed to broadcast to topic '{topic}': {reason}")]
    BroadcastError { topic: String, reason: String },

    /// Authentication required
    #[error("Authentication required")]
    AuthenticationRequired,

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid subscription topic
    #[error("Invalid subscription topic: {0}")]
    InvalidTopic(String),

    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Custom error
    #[error("{0}")]
    Custom(String),
}

impl BidirectionalError {
    /// Create a WebSocket error
    pub fn websocket<E: std::fmt::Display>(error: E) -> Self {
        Self::WebSocketError(error.to_string())
    }

    /// Create an internal error
    pub fn internal<E: std::fmt::Display>(error: E) -> Self {
        Self::InternalError(error.to_string())
    }

    /// Create a custom error
    pub fn custom<E: std::fmt::Display>(error: E) -> Self {
        Self::Custom(error.to_string())
    }
}
