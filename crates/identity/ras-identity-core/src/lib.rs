//! Core identity provider traits and types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Unsupported authentication method")]
    UnsupportedMethod,

    #[error("Invalid authentication payload")]
    InvalidPayload,

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type IdentityResult<T> = Result<T, IdentityError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedIdentity {
    pub provider_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[async_trait]
pub trait IdentityProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn verify(&self, auth_payload: serde_json::Value) -> IdentityResult<VerifiedIdentity>;
}

#[async_trait]
pub trait UserPermissions: Send + Sync {
    async fn get_permissions(&self, identity: &VerifiedIdentity) -> IdentityResult<Vec<String>>;
}

/// A default implementation that returns no permissions
pub struct NoopPermissions;

#[async_trait]
impl UserPermissions for NoopPermissions {
    async fn get_permissions(&self, _identity: &VerifiedIdentity) -> IdentityResult<Vec<String>> {
        Ok(Vec::new())
    }
}

/// A static permissions provider that returns the same permissions for all users
pub struct StaticPermissions {
    permissions: Vec<String>,
}

impl StaticPermissions {
    pub fn new(permissions: Vec<String>) -> Self {
        Self { permissions }
    }
}

#[async_trait]
impl UserPermissions for StaticPermissions {
    async fn get_permissions(&self, _identity: &VerifiedIdentity) -> IdentityResult<Vec<String>> {
        Ok(self.permissions.clone())
    }
}
