//! Authentication provider trait and related types for JSON-RPC services.
//!
//! This module defines the core authentication abstraction that allows different
//! authentication schemes to be plugged into JSON-RPC endpoints. The design is
//! async-first and integrates seamlessly with the Axum web framework.

use std::future::Future;
use std::pin::Pin;

/// Result type for authentication operations.
pub type AuthResult<T> = Result<T, AuthError>;

/// Errors that can occur during authentication and authorization.
#[derive(Debug, Clone)]
pub enum AuthError {
    /// The provided credentials are invalid or missing.
    InvalidCredentials(String),
    
    /// The authentication token has expired.
    TokenExpired,
    
    /// The authenticated user lacks the required permissions.
    InsufficientPermissions {
        /// The permission that was required.
        required: String,
        /// The permissions the user actually has.
        actual: Vec<String>,
    },
    
    /// The authentication scheme is not supported.
    UnsupportedScheme(String),
    
    /// An internal error occurred during authentication.
    Internal(String),
    
    /// Custom error for provider-specific failures.
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials(msg) => write!(f, "Invalid credentials: {}", msg),
            AuthError::TokenExpired => write!(f, "Token has expired"),
            AuthError::InsufficientPermissions { required, actual } => {
                write!(f, "Insufficient permissions. Required: {}, Actual: {:?}", required, actual)
            }
            AuthError::UnsupportedScheme(scheme) => write!(f, "Unsupported auth scheme: {}", scheme),
            AuthError::Internal(msg) => write!(f, "Internal auth error: {}", msg),
            AuthError::Custom(err) => write!(f, "Custom auth error: {}", err),
        }
    }
}

impl std::error::Error for AuthError {}

/// Represents an authenticated context with user information and permissions.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Unique identifier for the authenticated entity.
    pub subject: String,
    
    /// The authentication scheme used (e.g., "Bearer", "ApiKey").
    pub scheme: String,
    
    /// List of permissions granted to this authenticated entity.
    pub permissions: Vec<String>,
    
    /// Additional metadata about the authenticated entity.
    /// This can include things like email, organization ID, etc.
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    
    /// When this authentication context expires, if applicable.
    pub expires_at: Option<std::time::SystemTime>,
}

impl AuthContext {
    /// Check if this context has a specific permission.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }
    
    /// Check if this context has all of the specified permissions.
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|&p| self.has_permission(p))
    }
    
    /// Check if this context has any of the specified permissions.
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|&p| self.has_permission(p))
    }
    
    /// Check if this authentication context has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at <= std::time::SystemTime::now()
        } else {
            false
        }
    }
}

/// Raw authentication data extracted from a request.
#[derive(Debug, Clone)]
pub struct AuthCredentials {
    /// The authentication scheme (e.g., "Bearer", "ApiKey").
    pub scheme: String,
    
    /// The raw credential value (e.g., token, API key).
    pub value: String,
    
    /// Additional parameters that might be part of the auth header.
    pub parameters: std::collections::HashMap<String, String>,
}

/// Trait for implementing authentication providers.
///
/// This trait is designed to be implemented by different authentication schemes
/// such as JWT, API keys, OAuth, etc. It provides a unified interface for
/// validating credentials and checking permissions.
///
/// # Example
///
/// ```ignore
/// struct JwtAuthProvider {
///     secret: String,
/// }
///
/// #[async_trait]
/// impl AuthProvider for JwtAuthProvider {
///     async fn validate(&self, credentials: AuthCredentials) -> AuthResult<AuthContext> {
///         // Validate JWT token and extract claims
///         todo!()
///     }
///     
///     async fn check_permission(
///         &self,
///         context: &AuthContext,
///         permission: &str
///     ) -> AuthResult<()> {
///         if context.has_permission(permission) {
///             Ok(())
///         } else {
///             Err(AuthError::InsufficientPermissions {
///                 required: permission.to_string(),
///                 actual: context.permissions.clone(),
///             })
///         }
///     }
/// }
/// ```
pub trait AuthProvider: Send + Sync + 'static {
    /// Validate the provided credentials and return an authenticated context.
    ///
    /// This method should:
    /// - Verify the credentials are valid
    /// - Extract user/subject information
    /// - Load the associated permissions
    /// - Return an AuthContext with all relevant information
    fn validate(
        &self,
        credentials: AuthCredentials,
    ) -> Pin<Box<dyn Future<Output = AuthResult<AuthContext>> + Send + '_>>;
    
    /// Check if the authenticated context has a specific permission.
    ///
    /// This method can be used for fine-grained authorization checks.
    /// The default implementation checks the permissions list in the context,
    /// but providers can override this for more complex authorization logic.
    fn check_permission(
        &self,
        context: &AuthContext,
        permission: &str,
    ) -> Pin<Box<dyn Future<Output = AuthResult<()>> + Send + '_>> {
        Box::pin(async move {
            if context.is_expired() {
                return Err(AuthError::TokenExpired);
            }
            
            if context.has_permission(permission) {
                Ok(())
            } else {
                Err(AuthError::InsufficientPermissions {
                    required: permission.to_string(),
                    actual: context.permissions.clone(),
                })
            }
        })
    }
    
    /// Check if the authenticated context has all of the specified permissions.
    fn check_all_permissions(
        &self,
        context: &AuthContext,
        permissions: &[&str],
    ) -> Pin<Box<dyn Future<Output = AuthResult<()>> + Send + '_>> {
        Box::pin(async move {
            if context.is_expired() {
                return Err(AuthError::TokenExpired);
            }
            
            for permission in permissions {
                if !context.has_permission(permission) {
                    return Err(AuthError::InsufficientPermissions {
                        required: permission.to_string(),
                        actual: context.permissions.clone(),
                    });
                }
            }
            Ok(())
        })
    }
    
    /// Check if the authenticated context has any of the specified permissions.
    fn check_any_permission(
        &self,
        context: &AuthContext,
        permissions: &[&str],
    ) -> Pin<Box<dyn Future<Output = AuthResult<()>> + Send + '_>> {
        Box::pin(async move {
            if context.is_expired() {
                return Err(AuthError::TokenExpired);
            }
            
            if context.has_any_permission(permissions) {
                Ok(())
            } else {
                Err(AuthError::InsufficientPermissions {
                    required: permissions.join(" OR "),
                    actual: context.permissions.clone(),
                })
            }
        })
    }
    
    /// Refresh an authentication context if supported by this provider.
    ///
    /// This is optional and providers can return an error if refresh is not supported.
    fn refresh(
        &self,
        context: &AuthContext,
    ) -> Pin<Box<dyn Future<Output = AuthResult<AuthContext>> + Send + '_>> {
        Box::pin(async move {
            Err(AuthError::UnsupportedScheme(
                "Token refresh not supported by this provider".to_string()
            ))
        })
    }
}

/// Extension trait for extracting authentication from Axum requests.
///
/// This trait will be implemented for axum::http::Request to provide
/// convenient methods for extracting authentication credentials.
pub trait AuthExtractor {
    /// Extract authentication credentials from the request.
    fn extract_auth(&self) -> Option<AuthCredentials>;
}

/// Middleware configuration for authentication.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether to allow anonymous access when no credentials are provided.
    pub allow_anonymous: bool,
    
    /// Required permissions for accessing the endpoint.
    pub required_permissions: Vec<String>,
    
    /// Whether to require all permissions or just one of them.
    pub require_all: bool,
    
    /// Custom header name for authentication (default: "Authorization").
    pub header_name: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            allow_anonymous: false,
            required_permissions: Vec::new(),
            require_all: true,
            header_name: "Authorization".to_string(),
        }
    }
}

impl AuthConfig {
    /// Create a new auth config that requires authentication but no specific permissions.
    pub fn authenticated() -> Self {
        Self::default()
    }
    
    /// Create a new auth config that allows anonymous access.
    pub fn anonymous() -> Self {
        Self {
            allow_anonymous: true,
            ..Default::default()
        }
    }
    
    /// Create a new auth config that requires a single permission.
    pub fn with_permission(permission: impl Into<String>) -> Self {
        Self {
            required_permissions: vec![permission.into()],
            ..Default::default()
        }
    }
    
    /// Create a new auth config that requires all of the specified permissions.
    pub fn with_all_permissions(permissions: Vec<String>) -> Self {
        Self {
            required_permissions: permissions,
            require_all: true,
            ..Default::default()
        }
    }
    
    /// Create a new auth config that requires any of the specified permissions.
    pub fn with_any_permission(permissions: Vec<String>) -> Self {
        Self {
            required_permissions: permissions,
            require_all: false,
            ..Default::default()
        }
    }
}

/// Type alias for a boxed auth provider.
pub type BoxedAuthProvider = Box<dyn AuthProvider>;

/// Builder for creating auth providers with common configurations.
pub struct AuthProviderBuilder<T> {
    provider: T,
}

impl<T: AuthProvider> AuthProviderBuilder<T> {
    /// Create a new builder with the given provider.
    pub fn new(provider: T) -> Self {
        Self { provider }
    }
    
    /// Build the auth provider.
    pub fn build(self) -> T {
        self.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_auth_context_permissions() {
        let context = AuthContext {
            subject: "user123".to_string(),
            scheme: "Bearer".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            metadata: std::collections::HashMap::new(),
            expires_at: None,
        };
        
        assert!(context.has_permission("read"));
        assert!(context.has_permission("write"));
        assert!(!context.has_permission("delete"));
        
        assert!(context.has_all_permissions(&["read", "write"]));
        assert!(!context.has_all_permissions(&["read", "write", "delete"]));
        
        assert!(context.has_any_permission(&["read", "delete"]));
        assert!(!context.has_any_permission(&["delete", "admin"]));
    }
    
    #[test]
    fn test_auth_context_expiration() {
        let expired_context = AuthContext {
            subject: "user123".to_string(),
            scheme: "Bearer".to_string(),
            permissions: vec![],
            metadata: std::collections::HashMap::new(),
            expires_at: Some(std::time::SystemTime::now() - std::time::Duration::from_secs(60)),
        };
        
        assert!(expired_context.is_expired());
        
        let valid_context = AuthContext {
            subject: "user123".to_string(),
            scheme: "Bearer".to_string(),
            permissions: vec![],
            metadata: std::collections::HashMap::new(),
            expires_at: Some(std::time::SystemTime::now() + std::time::Duration::from_secs(60)),
        };
        
        assert!(!valid_context.is_expired());
    }
}