//! Authentication and authorization traits for JSON-RPC services.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during authentication or authorization.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum AuthError {
    /// The provided token is invalid or malformed.
    #[error("Invalid token")]
    InvalidToken,

    /// The token has expired.
    #[error("Token expired")]
    TokenExpired,

    /// The token does not have the required permissions.
    #[error("Insufficient permissions: required {required:?}, has {has:?}")]
    InsufficientPermissions {
        required: Vec<String>,
        has: Vec<String>,
    },

    /// Authentication is required but no token was provided.
    #[error("Authentication required")]
    AuthenticationRequired,

    /// An internal error occurred during authentication.
    #[error("Authentication error: {0}")]
    Internal(String),
}

/// Represents an authenticated user with their permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    /// Unique identifier for the user.
    pub user_id: String,

    /// Set of permissions granted to this user.
    pub permissions: HashSet<String>,

    /// Optional additional metadata about the user.
    pub metadata: Option<serde_json::Value>,
}

/// Result type for authentication operations.
pub type AuthResult<T = AuthenticatedUser> = Result<T, AuthError>;

/// Boxed future for async authentication operations.
pub type AuthFuture<'a, T = AuthenticatedUser> =
    Pin<Box<dyn Future<Output = AuthResult<T>> + Send + 'a>>;

/// Trait for implementing authentication providers.
///
/// This trait allows for flexible authentication mechanisms while providing
/// a consistent interface for the JSON-RPC service layer.
pub trait AuthProvider: Send + Sync + 'static {
    /// Validates a token and returns the authenticated user.
    ///
    /// # Arguments
    /// * `token` - The authentication token to validate (e.g., JWT, API key)
    ///
    /// # Returns
    /// * `Ok(AuthenticatedUser)` if the token is valid
    /// * `Err(AuthError)` if validation fails
    fn authenticate(&self, token: String) -> AuthFuture<'_>;

    /// Checks if the authenticated user has the required permissions.
    ///
    /// # Arguments
    /// * `user` - The authenticated user
    /// * `required_permissions` - List of permissions that are required
    ///
    /// # Returns
    /// * `Ok(())` if the user has all required permissions
    /// * `Err(AuthError::InsufficientPermissions)` if any permission is missing
    fn check_permissions(
        &self,
        user: &AuthenticatedUser,
        required_permissions: &[String],
    ) -> AuthResult<()> {
        let missing_permissions: Vec<String> = required_permissions
            .iter()
            .filter(|perm| !user.permissions.contains(*perm))
            .cloned()
            .collect();

        if missing_permissions.is_empty() {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions {
                required: required_permissions.to_vec(),
                has: user.permissions.iter().cloned().collect(),
            })
        }
    }

    /// Validates a token and checks permissions in one operation.
    ///
    /// This is a convenience method that combines authentication and authorization.
    ///
    /// # Arguments
    /// * `token` - The authentication token to validate
    /// * `required_permissions` - List of permissions that are required
    ///
    /// # Returns
    /// * `Ok(AuthenticatedUser)` if the token is valid and has required permissions
    /// * `Err(AuthError)` if validation or authorization fails
    fn authenticate_and_authorize(
        &self,
        token: String,
        required_permissions: Vec<String>,
    ) -> AuthFuture<'_> {
        Box::pin(async move {
            let user = self.authenticate(token).await?;
            self.check_permissions(&user, &required_permissions)?;
            Ok(user)
        })
    }
}

/// Extension trait for working with optional tokens.
pub trait AuthProviderExt: AuthProvider {
    /// Validates an optional token, returning None if no token is provided.
    ///
    /// # Arguments
    /// * `token` - Optional authentication token
    ///
    /// # Returns
    /// * `Ok(Some(AuthenticatedUser))` if token is provided and valid
    /// * `Ok(None)` if no token is provided
    /// * `Err(AuthError)` if token is provided but invalid
    fn authenticate_optional(
        &self,
        token: Option<String>,
    ) -> AuthFuture<'_, Option<AuthenticatedUser>> {
        Box::pin(async move {
            match token {
                Some(token) => self.authenticate(token).await.map(Some),
                None => Ok(None),
            }
        })
    }
}

impl<T: AuthProvider> AuthProviderExt for T {}
