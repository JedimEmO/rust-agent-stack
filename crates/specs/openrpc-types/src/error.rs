//! Error types for OpenRPC specification validation and processing.

use thiserror::Error;

/// Errors that can occur when working with OpenRPC specifications.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum OpenRpcError {
    /// Validation error when OpenRPC specification constraints are violated
    #[error("Validation error: {message}")]
    ValidationError {
        /// Human-readable error message
        message: String,
        /// Optional field path where the error occurred
        field_path: Option<String>,
    },

    /// Error when parsing or serializing JSON
    #[error("JSON error: {message}")]
    JsonError {
        /// JSON parsing/serialization error message
        message: String,
    },

    /// Error when resolving references ($ref)
    #[error("Reference resolution error: {message}")]
    ReferenceError {
        /// Reference resolution error message
        message: String,
        /// The reference string that failed to resolve
        reference: String,
    },

    /// Error when a required field is missing
    #[error("Missing required field: {field_name}")]
    MissingField {
        /// Name of the missing required field
        field_name: String,
    },

    /// Error when a field has an invalid value
    #[error("Invalid field value for '{field_name}': {message}")]
    InvalidField {
        /// Name of the field with invalid value
        field_name: String,
        /// Description of why the value is invalid
        message: String,
    },

    /// Error when an object has duplicate keys that should be unique
    #[error("Duplicate key '{key}' found in {context}")]
    DuplicateKey {
        /// The duplicate key name
        key: String,
        /// Context where the duplicate was found
        context: String,
    },

    /// Error when URL format is invalid
    #[error("Invalid URL format: {url}")]
    InvalidUrl {
        /// The invalid URL string
        url: String,
    },

    /// Error when email format is invalid
    #[error("Invalid email format: {email}")]
    InvalidEmail {
        /// The invalid email string
        email: String,
    },

    /// Error when regex pattern is invalid
    #[error("Invalid regex pattern: {pattern}")]
    InvalidRegex {
        /// The invalid regex pattern
        pattern: String,
    },

    /// Error when OpenRPC version is unsupported
    #[error("Unsupported OpenRPC version: {version}")]
    UnsupportedVersion {
        /// The unsupported version string
        version: String,
    },

    /// Error when JSON Schema Draft 7 constraints are violated
    #[error("JSON Schema validation error: {message}")]
    SchemaError {
        /// Schema validation error message
        message: String,
        /// Optional schema path where the error occurred
        schema_path: Option<String>,
    },
}

impl OpenRpcError {
    /// Create a new validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
            field_path: None,
        }
    }

    /// Create a new validation error with field path
    pub fn validation_with_path(message: impl Into<String>, field_path: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
            field_path: Some(field_path.into()),
        }
    }

    /// Create a new JSON error
    pub fn json(message: impl Into<String>) -> Self {
        Self::JsonError {
            message: message.into(),
        }
    }

    /// Create a new reference resolution error
    pub fn reference(message: impl Into<String>, reference: impl Into<String>) -> Self {
        Self::ReferenceError {
            message: message.into(),
            reference: reference.into(),
        }
    }

    /// Create a new missing field error
    pub fn missing_field(field_name: impl Into<String>) -> Self {
        Self::MissingField {
            field_name: field_name.into(),
        }
    }

    /// Create a new invalid field error
    pub fn invalid_field(field_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidField {
            field_name: field_name.into(),
            message: message.into(),
        }
    }

    /// Create a new duplicate key error
    pub fn duplicate_key(key: impl Into<String>, context: impl Into<String>) -> Self {
        Self::DuplicateKey {
            key: key.into(),
            context: context.into(),
        }
    }

    /// Create a new invalid URL error
    pub fn invalid_url(url: impl Into<String>) -> Self {
        Self::InvalidUrl { url: url.into() }
    }

    /// Create a new invalid email error
    pub fn invalid_email(email: impl Into<String>) -> Self {
        Self::InvalidEmail {
            email: email.into(),
        }
    }

    /// Create a new invalid regex error
    pub fn invalid_regex(pattern: impl Into<String>) -> Self {
        Self::InvalidRegex {
            pattern: pattern.into(),
        }
    }

    /// Create a new unsupported version error
    pub fn unsupported_version(version: impl Into<String>) -> Self {
        Self::UnsupportedVersion {
            version: version.into(),
        }
    }

    /// Create a new schema validation error
    pub fn schema(message: impl Into<String>) -> Self {
        Self::SchemaError {
            message: message.into(),
            schema_path: None,
        }
    }

    /// Create a new schema validation error with path
    pub fn schema_with_path(message: impl Into<String>, schema_path: impl Into<String>) -> Self {
        Self::SchemaError {
            message: message.into(),
            schema_path: Some(schema_path.into()),
        }
    }
}

impl From<serde_json::Error> for OpenRpcError {
    fn from(err: serde_json::Error) -> Self {
        Self::json(err.to_string())
    }
}

/// Result type for OpenRPC operations
pub type OpenRpcResult<T> = Result<T, OpenRpcError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = OpenRpcError::validation("test message");
        assert!(matches!(err, OpenRpcError::ValidationError { .. }));

        let err = OpenRpcError::validation_with_path("test", "field.path");
        if let OpenRpcError::ValidationError { field_path, .. } = err {
            assert_eq!(field_path, Some("field.path".to_string()));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_error_display() {
        let err = OpenRpcError::validation("test validation error");
        assert_eq!(err.to_string(), "Validation error: test validation error");

        let err = OpenRpcError::missing_field("required_field");
        assert_eq!(err.to_string(), "Missing required field: required_field");
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_err.is_err());

        let openrpc_err: OpenRpcError = json_err.unwrap_err().into();
        assert!(matches!(openrpc_err, OpenRpcError::JsonError { .. }));
    }
}
