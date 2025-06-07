//! Local user identity provider with username/password authentication.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use async_trait::async_trait;
use rand_core::OsRng;
use rust_identity_core::{IdentityError, IdentityProvider, IdentityResult, VerifiedIdentity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUser {
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalAuthPayload {
    pub username: String,
    pub password: String,
}

pub struct LocalUserProvider {
    users: Arc<RwLock<HashMap<String, LocalUser>>>,
}

impl LocalUserProvider {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_user(
        &self,
        username: String,
        password: String,
        email: Option<String>,
        display_name: Option<String>,
    ) -> Result<(), argon2::password_hash::Error> {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        let user = LocalUser {
            username: username.clone(),
            password_hash,
            email,
            display_name,
            metadata: None,
        };

        let mut users = self.users.write().await;
        users.insert(username, user);
        Ok(())
    }

    pub async fn remove_user(&self, username: &str) -> Option<LocalUser> {
        let mut users = self.users.write().await;
        users.remove(username)
    }

    async fn verify_user(&self, username: &str, password: &str) -> IdentityResult<LocalUser> {
        let users = self.users.read().await;
        let user = users
            .get(username)
            .ok_or_else(|| IdentityError::UserNotFound(username.to_string()))?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| IdentityError::ProviderError(e.to_string()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| IdentityError::InvalidCredentials)?;

        Ok(user.clone())
    }
}

impl Default for LocalUserProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IdentityProvider for LocalUserProvider {
    fn provider_id(&self) -> &str {
        "local"
    }

    async fn verify(&self, auth_payload: serde_json::Value) -> IdentityResult<VerifiedIdentity> {
        let payload: LocalAuthPayload =
            serde_json::from_value(auth_payload).map_err(|_| IdentityError::InvalidPayload)?;

        let user = self
            .verify_user(&payload.username, &payload.password)
            .await?;

        Ok(VerifiedIdentity {
            provider_id: self.provider_id().to_string(),
            subject: user.username,
            email: user.email,
            display_name: user.display_name,
            metadata: user.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_user_auth() {
        let provider = LocalUserProvider::new();

        provider
            .add_user(
                "testuser".to_string(),
                "password123".to_string(),
                Some("test@example.com".to_string()),
                Some("Test User".to_string()),
            )
            .await
            .unwrap();

        let auth_payload = serde_json::json!({
            "username": "testuser",
            "password": "password123"
        });

        let identity = provider.verify(auth_payload).await.unwrap();
        assert_eq!(identity.subject, "testuser");
        assert_eq!(identity.email.as_deref(), Some("test@example.com"));

        let bad_payload = serde_json::json!({
            "username": "testuser",
            "password": "wrongpassword"
        });

        assert!(provider.verify(bad_payload).await.is_err());
    }
}
