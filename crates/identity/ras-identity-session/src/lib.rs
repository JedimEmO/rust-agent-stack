//! Session management with JWT token generation and validation.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use ras_auth_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_identity_core::{IdentityError, IdentityProvider, UserPermissions};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Identity error: {0}")]
    IdentityError(#[from] IdentityError),

    #[error("Session not found")]
    SessionNotFound,

    #[error("Invalid session")]
    InvalidSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub provider_id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub permissions: HashSet<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub jwt_secret: String,
    pub jwt_ttl: Duration,
    pub refresh_enabled: bool,
    pub algorithm: Algorithm,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me-in-production".to_string(),
            jwt_ttl: Duration::hours(24),
            refresh_enabled: true,
            algorithm: Algorithm::HS256,
        }
    }
}

pub struct SessionService {
    config: SessionConfig,
    providers: Arc<RwLock<HashMap<String, Box<dyn IdentityProvider>>>>,
    active_sessions: Arc<RwLock<HashMap<String, JwtClaims>>>,
    permissions_provider: Option<Arc<dyn UserPermissions>>,
}

impl SessionService {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            permissions_provider: None,
        }
    }

    pub fn with_permissions(mut self, provider: Arc<dyn UserPermissions>) -> Self {
        self.permissions_provider = Some(provider);
        self
    }

    pub fn set_permissions_provider(&mut self, provider: Arc<dyn UserPermissions>) {
        self.permissions_provider = Some(provider);
    }

    pub async fn register_provider(&self, provider: Box<dyn IdentityProvider>) {
        let mut providers = self.providers.write().await;
        providers.insert(provider.provider_id().to_string(), provider);
    }

    pub async fn begin_session(
        &self,
        provider_id: &str,
        auth_payload: serde_json::Value,
    ) -> Result<String, SessionError> {
        let providers = self.providers.read().await;
        let provider = providers
            .get(provider_id)
            .ok_or_else(|| IdentityError::ProviderNotFound(provider_id.to_string()))?;

        let identity = provider.verify(auth_payload).await?;

        let now = Utc::now();
        let exp = now + self.config.jwt_ttl;
        let jti = Uuid::new_v4().to_string();

        let permissions = if let Some(ref perm_provider) = self.permissions_provider {
            perm_provider.get_permissions(&identity).await?
        } else {
            Vec::new()
        };

        let claims = JwtClaims {
            sub: identity.subject.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: jti.clone(),
            provider_id: identity.provider_id.clone(),
            email: identity.email.clone(),
            display_name: identity.display_name.clone(),
            permissions: permissions.into_iter().collect(),
            metadata: identity.metadata,
        };

        let mut sessions = self.active_sessions.write().await;
        sessions.insert(jti.clone(), claims.clone());

        let token = encode(
            &Header::new(self.config.algorithm),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )?;

        Ok(token)
    }

    pub async fn verify_session(&self, token: &str) -> Result<JwtClaims, SessionError> {
        let token_data = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::new(self.config.algorithm),
        )?;

        let sessions = self.active_sessions.read().await;
        if !sessions.contains_key(&token_data.claims.jti) {
            return Err(SessionError::SessionNotFound);
        }

        Ok(token_data.claims)
    }

    pub async fn end_session(&self, jti: &str) -> Option<JwtClaims> {
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(jti)
    }
}

#[derive(Clone)]
pub struct JwtAuthProvider {
    session_service: Arc<SessionService>,
}

impl JwtAuthProvider {
    pub fn new(session_service: Arc<SessionService>) -> Self {
        Self { session_service }
    }
}

#[async_trait]
impl AuthProvider for JwtAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            let claims =
                self.session_service
                    .verify_session(&token)
                    .await
                    .map_err(|e| match e {
                        SessionError::JwtError(jwt_err) => {
                            if jwt_err.to_string().contains("expired") {
                                AuthError::TokenExpired
                            } else {
                                AuthError::InvalidToken
                            }
                        }
                        _ => AuthError::InvalidToken,
                    })?;

            Ok(AuthenticatedUser {
                user_id: claims.sub,
                permissions: claims.permissions,
                metadata: claims.metadata,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ras_identity_core::StaticPermissions;
    use ras_identity_local::LocalUserProvider;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let config = SessionConfig::default();
        let session_service = SessionService::new(config);

        let local_provider = LocalUserProvider::new();
        local_provider
            .add_user(
                "testuser".to_string(),
                "password123".to_string(),
                Some("test@example.com".to_string()),
                Some("Test User".to_string()),
            )
            .await
            .unwrap();

        session_service
            .register_provider(Box::new(local_provider))
            .await;

        let auth_payload = serde_json::json!({
            "username": "testuser",
            "password": "password123"
        });

        let token = session_service
            .begin_session("local", auth_payload)
            .await
            .unwrap();

        let claims = session_service.verify_session(&token).await.unwrap();
        assert_eq!(claims.sub, "testuser");
        assert_eq!(claims.provider_id, "local");
        assert!(claims.permissions.is_empty());

        session_service.end_session(&claims.jti).await;

        assert!(session_service.verify_session(&token).await.is_err());
    }

    #[tokio::test]
    async fn test_session_with_permissions() {
        let config = SessionConfig::default();
        let permissions_provider = Arc::new(StaticPermissions::new(vec![
            "read".to_string(),
            "write".to_string(),
        ]));
        let session_service = SessionService::new(config).with_permissions(permissions_provider);

        let local_provider = LocalUserProvider::new();
        local_provider
            .add_user(
                "admin".to_string(),
                "admin123".to_string(),
                Some("admin@example.com".to_string()),
                Some("Admin User".to_string()),
            )
            .await
            .unwrap();

        session_service
            .register_provider(Box::new(local_provider))
            .await;

        let auth_payload = serde_json::json!({
            "username": "admin",
            "password": "admin123"
        });

        let token = session_service
            .begin_session("local", auth_payload)
            .await
            .unwrap();

        let claims = session_service.verify_session(&token).await.unwrap();
        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.permissions.len(), 2);
        assert!(claims.permissions.contains("read"));
        assert!(claims.permissions.contains("write"));
    }
}
