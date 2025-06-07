//! OAuth2 identity provider implementation.

use async_trait::async_trait;
use rust_identity_core::{IdentityError, IdentityProvider, IdentityResult, VerifiedIdentity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuth2AuthPayload {
    pub access_token: String,
    pub provider: String,
}

#[derive(Debug, Clone)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: String,
}

pub struct OAuth2Provider {
    #[allow(dead_code)]
    config: OAuth2Config,
    provider_name: String,
}

impl OAuth2Provider {
    pub fn new(provider_name: String, config: OAuth2Config) -> Self {
        Self {
            config,
            provider_name,
        }
    }

    async fn verify_token(&self, _access_token: &str) -> IdentityResult<OAuth2UserInfo> {
        // TODO: Implement actual OAuth2 token verification
        // This would typically involve:
        // 1. Making a request to the userinfo endpoint with the access token
        // 2. Validating the response
        // 3. Extracting user information

        // For now, return a placeholder error
        Err(IdentityError::ProviderError(
            "OAuth2 verification not yet implemented".to_string(),
        ))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OAuth2UserInfo {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

#[async_trait]
impl IdentityProvider for OAuth2Provider {
    fn provider_id(&self) -> &str {
        &self.provider_name
    }

    async fn verify(&self, auth_payload: serde_json::Value) -> IdentityResult<VerifiedIdentity> {
        let payload: OAuth2AuthPayload =
            serde_json::from_value(auth_payload).map_err(|_| IdentityError::InvalidPayload)?;

        if payload.provider != self.provider_name {
            return Err(IdentityError::ProviderError(format!(
                "Provider mismatch: expected {}, got {}",
                self.provider_name, payload.provider
            )));
        }

        let user_info = self.verify_token(&payload.access_token).await?;

        Ok(VerifiedIdentity {
            provider_id: self.provider_id().to_string(),
            subject: user_info.sub,
            email: user_info.email,
            display_name: user_info.name,
            metadata: Some(serde_json::json!({
                "picture": user_info.picture,
            })),
        })
    }
}
