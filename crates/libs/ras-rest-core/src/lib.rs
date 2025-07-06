//! Core types and traits for REST services in Rust Agent Stack.
//!
//! This crate provides the runtime types needed for REST services, including:
//! - `RestResult`, `RestResponse`, and `RestError` for explicit HTTP status code handling
//! - Re-exports of authentication types from `ras-auth-core`

use thiserror::Error;

// Re-export authentication types for convenience
pub use ras_auth_core::{AuthError, AuthProvider, AuthResult, AuthenticatedUser};

/// Result type for REST handlers that allows explicit HTTP status codes.
pub type RestResult<T> = Result<RestResponse<T>, RestError>;

/// Successful REST response wrapper.
#[derive(Debug, Clone)]
pub struct RestResponse<T> {
    /// HTTP status code (default: 200)
    pub status: u16,
    /// Response body
    pub body: T,
}

impl<T> RestResponse<T> {
    /// Create a 200 OK response.
    pub fn ok(body: T) -> Self {
        Self { status: 200, body }
    }

    /// Create a 201 Created response.
    pub fn created(body: T) -> Self {
        Self { status: 201, body }
    }

    /// Create a 202 Accepted response.
    pub fn accepted(body: T) -> Self {
        Self { status: 202, body }
    }

    /// Create a 204 No Content response (requires T to be ()).
    pub fn no_content() -> Self
    where
        T: Default,
    {
        Self {
            status: 204,
            body: T::default(),
        }
    }

    /// Create a response with a custom status code.
    pub fn with_status(status: u16, body: T) -> Self {
        Self { status, body }
    }
}

/// REST error with explicit HTTP status code.
#[derive(Debug, Error)]
#[error("HTTP {status}: {message}")]
pub struct RestError {
    /// HTTP status code
    pub status: u16,
    /// Error message to send to client
    pub message: String,
    /// Optional internal error for logging (not sent to client)
    #[source]
    pub internal_error: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl RestError {
    /// Create a new REST error.
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            internal_error: None,
        }
    }

    /// Create a new REST error with internal error details for logging.
    pub fn with_internal<E>(status: u16, message: impl Into<String>, internal: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            status,
            message: message.into(),
            internal_error: Some(Box::new(internal)),
        }
    }

    /// Create a 400 Bad Request error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, message)
    }

    /// Create a 401 Unauthorized error.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(401, message)
    }

    /// Create a 403 Forbidden error.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, message)
    }

    /// Create a 404 Not Found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, message)
    }

    /// Create a 409 Conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(409, message)
    }

    /// Create a 422 Unprocessable Entity error.
    pub fn unprocessable_entity(message: impl Into<String>) -> Self {
        Self::new(422, message)
    }

    /// Create a 500 Internal Server Error.
    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::new(500, message)
    }

    /// Create a 502 Bad Gateway error.
    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self::new(502, message)
    }

    /// Create a 503 Service Unavailable error.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(503, message)
    }
}

/// Helper trait to convert various error types to RestError.
pub trait IntoRestError {
    /// Convert this error into a RestError.
    fn into_rest_error(self) -> RestError;
}

impl<E: std::error::Error + Send + Sync + 'static> IntoRestError for E {
    fn into_rest_error(self) -> RestError {
        RestError::with_internal(500, "Internal server error", self)
    }
}

/// Extension trait for Result types to easily convert errors to RestError.
pub trait RestResultExt<T> {
    /// Convert any error to a RestError with a 500 status code.
    fn internal_server_error(self) -> RestResult<T>;

    /// Convert any error to a RestError with a custom status code and message.
    fn rest_error(self, status: u16, message: impl Into<String>) -> RestResult<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> RestResultExt<T> for Result<T, E> {
    fn internal_server_error(self) -> RestResult<T> {
        self.map(RestResponse::ok)
            .map_err(|e| RestError::with_internal(500, "Internal server error", e))
    }

    fn rest_error(self, status: u16, message: impl Into<String>) -> RestResult<T> {
        let msg = message.into();
        self.map(RestResponse::ok)
            .map_err(|e| RestError::with_internal(status, msg, e))
    }
}
