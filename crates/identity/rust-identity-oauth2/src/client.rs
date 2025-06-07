//! OAuth2 client implementation with PKCE support.

use crate::config::OAuth2ProviderConfig;
use crate::error::{OAuth2Error, OAuth2Result};
use crate::state::{OAuth2State, OAuth2StateStore};
use crate::types::{AuthorizationResponse, TokenResponse, UserInfoResponse};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{Rng, thread_rng};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};
use url::Url;

/// PKCE code challenge and verifier
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

impl Default for PkceChallenge {
    fn default() -> Self {
        Self::new()
    }
}

impl PkceChallenge {
    /// Generate a new PKCE challenge
    pub fn new() -> Self {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        Self {
            code_verifier,
            code_challenge,
            code_challenge_method: "S256".to_string(),
        }
    }

    fn generate_code_verifier() -> String {
        let mut rng = thread_rng();
        let bytes: Vec<u8> = (0..64).map(|_| rng.r#gen::<u8>()).collect();
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn generate_code_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let result = hasher.finalize();
        URL_SAFE_NO_PAD.encode(result)
    }
}

/// OAuth2 client for handling authorization flows
#[derive(Clone)]
pub struct OAuth2Client {
    http_client: Client,
    state_store: Arc<dyn OAuth2StateStore>,
    state_ttl_seconds: u64,
}

impl OAuth2Client {
    pub fn new(
        state_store: Arc<dyn OAuth2StateStore>,
        state_ttl_seconds: u64,
        http_timeout_seconds: u64,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(http_timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            state_store,
            state_ttl_seconds,
        }
    }

    #[cfg(test)]
    pub fn state_store(&self) -> &Arc<dyn OAuth2StateStore> {
        &self.state_store
    }

    /// Generate authorization URL for a provider
    pub async fn generate_authorization_url(
        &self,
        provider_config: &OAuth2ProviderConfig,
        additional_params: HashMap<String, String>,
    ) -> OAuth2Result<(String, String)> {
        let mut url = Url::parse(&provider_config.authorization_endpoint)?;

        // Generate PKCE if enabled
        let pkce = if provider_config.use_pkce {
            Some(PkceChallenge::new())
        } else {
            None
        };

        // Create and store state
        let state = OAuth2State::new(
            provider_config.provider_id.clone(),
            provider_config.redirect_uri.clone(),
            pkce.as_ref().map(|p| p.code_verifier.clone()),
            self.state_ttl_seconds,
        );

        let state_param = state.state.clone();
        self.state_store.store(state).await?;

        // Build query parameters
        let mut params = url.query_pairs_mut();
        params.append_pair("response_type", "code");
        params.append_pair("client_id", &provider_config.client_id);
        params.append_pair("redirect_uri", &provider_config.redirect_uri);
        params.append_pair("state", &state_param);

        // Add scopes
        if !provider_config.scopes.is_empty() {
            params.append_pair("scope", &provider_config.scopes.join(" "));
        }

        // Add PKCE parameters
        if let Some(pkce) = &pkce {
            params.append_pair("code_challenge", &pkce.code_challenge);
            params.append_pair("code_challenge_method", &pkce.code_challenge_method);
        }

        // Add provider-specific parameters
        for (key, value) in &provider_config.auth_params {
            params.append_pair(key, value);
        }

        // Add additional parameters from the request
        for (key, value) in &additional_params {
            params.append_pair(key, value);
        }

        drop(params);

        let auth_url = url.to_string();
        debug!(
            "Generated authorization URL for provider {}",
            provider_config.provider_id
        );

        Ok((auth_url, state_param))
    }

    /// Handle OAuth2 callback and exchange code for tokens
    pub async fn handle_callback(
        &self,
        provider_config: &OAuth2ProviderConfig,
        callback_response: AuthorizationResponse,
    ) -> OAuth2Result<TokenResponse> {
        // Verify state
        let state = self.state_store.retrieve(&callback_response.state).await?;

        if state.provider_id != provider_config.provider_id {
            return Err(OAuth2Error::InvalidState);
        }

        // Check for errors in callback
        if let Some(error) = &callback_response.error {
            let error_desc = callback_response
                .error_description.as_deref()
                .unwrap_or("No description");
            return Err(OAuth2Error::CallbackError(format!(
                "{}: {}",
                error, error_desc
            )));
        }

        // Exchange authorization code for tokens
        let token_response = self
            .exchange_code(
                provider_config,
                &callback_response.code,
                state.code_verifier.as_deref(),
            )
            .await?;

        Ok(token_response)
    }

    /// Exchange authorization code for tokens
    async fn exchange_code(
        &self,
        provider_config: &OAuth2ProviderConfig,
        code: &str,
        code_verifier: Option<&str>,
    ) -> OAuth2Result<TokenResponse> {
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("client_id", &provider_config.client_id);
        params.insert("client_secret", &provider_config.client_secret);
        params.insert("redirect_uri", &provider_config.redirect_uri);

        // Add PKCE verifier if present
        if let Some(verifier) = code_verifier {
            params.insert("code_verifier", verifier);
        }

        let response = self
            .http_client
            .post(&provider_config.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Token exchange failed: {}", error_text);
            return Err(OAuth2Error::TokenExchangeFailed(error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| OAuth2Error::InvalidTokenResponse(e.to_string()))?;

        info!("Successfully exchanged code for tokens");
        Ok(token_response)
    }

    /// Get user info using access token
    pub async fn get_user_info(
        &self,
        provider_config: &OAuth2ProviderConfig,
        access_token: &str,
    ) -> OAuth2Result<UserInfoResponse> {
        let userinfo_endpoint = provider_config.userinfo_endpoint.as_ref().ok_or_else(|| {
            OAuth2Error::ConfigError("User info endpoint not configured".to_string())
        })?;

        let response = self
            .http_client
            .get(userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("User info request failed: {}", error_text);
            return Err(OAuth2Error::UserInfoFailed(error_text));
        }

        let user_info: UserInfoResponse = response
            .json()
            .await
            .map_err(|e| OAuth2Error::InvalidUserInfoResponse(e.to_string()))?;

        debug!(
            "Successfully retrieved user info for subject: {}",
            user_info.sub
        );
        Ok(user_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::InMemoryStateStore;

    #[test]
    fn test_pkce_generation() {
        let pkce1 = PkceChallenge::new();
        let pkce2 = PkceChallenge::new();

        // Verifiers should be different
        assert_ne!(pkce1.code_verifier, pkce2.code_verifier);

        // Challenges should be different
        assert_ne!(pkce1.code_challenge, pkce2.code_challenge);

        // Method should be S256
        assert_eq!(pkce1.code_challenge_method, "S256");

        // Verify the challenge is correctly generated
        let expected_challenge = PkceChallenge::generate_code_challenge(&pkce1.code_verifier);
        assert_eq!(pkce1.code_challenge, expected_challenge);
    }

    #[tokio::test]
    async fn test_authorization_url_generation() {
        let state_store = Arc::new(InMemoryStateStore::new());
        let client = OAuth2Client::new(state_store, 600, 30);

        let provider_config = OAuth2ProviderConfig {
            provider_id: "test_provider".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: "test_secret".to_string(),
            authorization_endpoint: "https://example.com/auth".to_string(),
            token_endpoint: "https://example.com/token".to_string(),
            userinfo_endpoint: Some("https://example.com/userinfo".to_string()),
            redirect_uri: "http://localhost:3000/callback".to_string(),
            scopes: vec!["openid".to_string(), "email".to_string()],
            auth_params: HashMap::new(),
            use_pkce: true,
            user_info_mapping: None,
        };

        let (auth_url, state) = client
            .generate_authorization_url(&provider_config, HashMap::new())
            .await
            .unwrap();

        // Verify URL structure
        let url = Url::parse(&auth_url).unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
        assert_eq!(url.path(), "/auth");

        // Verify query parameters
        let params: HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(params.get("response_type"), Some(&"code".into()));
        assert_eq!(params.get("client_id"), Some(&"test_client_id".into()));
        assert_eq!(
            params.get("redirect_uri"),
            Some(&"http://localhost:3000/callback".into())
        );
        assert_eq!(params.get("state"), Some(&state.into()));
        assert_eq!(params.get("scope"), Some(&"openid email".into()));
        assert!(params.contains_key("code_challenge"));
        assert_eq!(params.get("code_challenge_method"), Some(&"S256".into()));
    }
}
