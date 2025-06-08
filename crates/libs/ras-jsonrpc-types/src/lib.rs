//! JSON-RPC 2.0 protocol types and utilities.
//!
//! This crate provides type-safe representations of JSON-RPC 2.0 protocol
//! structures including requests, responses, and errors.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// The JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// The method name to call.
    pub method: String,

    /// Parameters for the method call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,

    /// Request identifier for matching responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// The JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// The result of the method call (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error information (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request identifier for matching with requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 error structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code indicating the type of error.
    pub code: i32,

    /// Human-readable error message.
    pub message: String,

    /// Additional error information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Standard JSON-RPC error codes.
pub mod error_codes {
    /// Invalid JSON was received by the server.
    pub const PARSE_ERROR: i32 = -32700;

    /// The JSON sent is not a valid Request object.
    pub const INVALID_REQUEST: i32 = -32600;

    /// The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;

    /// Internal JSON-RPC error.
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Authentication required.
    pub const AUTHENTICATION_REQUIRED: i32 = -32001;

    /// Insufficient permissions.
    pub const INSUFFICIENT_PERMISSIONS: i32 = -32002;

    /// Token expired.
    pub const TOKEN_EXPIRED: i32 = -32003;
}

impl JsonRpcRequest {
    /// Creates a new JSON-RPC request.
    pub fn new(
        method: String,
        params: Option<serde_json::Value>,
        id: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id,
        }
    }
}

impl JsonRpcResponse {
    /// Creates a successful JSON-RPC response.
    pub fn success(result: serde_json::Value, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Creates an error JSON-RPC response.
    pub fn error(error: JsonRpcError, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

impl JsonRpcError {
    /// Creates a new JSON-RPC error.
    pub fn new(code: i32, message: String, data: Option<serde_json::Value>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    /// Creates a parse error.
    pub fn parse_error() -> Self {
        Self::new(error_codes::PARSE_ERROR, "Parse error".to_string(), None)
    }

    /// Creates an invalid request error.
    pub fn invalid_request() -> Self {
        Self::new(
            error_codes::INVALID_REQUEST,
            "Invalid Request".to_string(),
            None,
        )
    }

    /// Creates a method not found error.
    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", method),
            None,
        )
    }

    /// Creates an invalid params error.
    pub fn invalid_params(message: String) -> Self {
        Self::new(error_codes::INVALID_PARAMS, message, None)
    }

    /// Creates an internal error.
    pub fn internal_error(message: String) -> Self {
        Self::new(error_codes::INTERNAL_ERROR, message, None)
    }

    /// Creates an authentication required error.
    pub fn authentication_required() -> Self {
        Self::new(
            error_codes::AUTHENTICATION_REQUIRED,
            "Authentication required".to_string(),
            None,
        )
    }

    /// Creates an insufficient permissions error.
    pub fn insufficient_permissions(required: Vec<String>, has: Vec<String>) -> Self {
        Self::new(
            error_codes::INSUFFICIENT_PERMISSIONS,
            "Insufficient permissions".to_string(),
            Some(serde_json::json!({
                "required": required,
                "has": has
            })),
        )
    }

    /// Creates a token expired error.
    pub fn token_expired() -> Self {
        Self::new(
            error_codes::TOKEN_EXPIRED,
            "Token expired".to_string(),
            None,
        )
    }
}
