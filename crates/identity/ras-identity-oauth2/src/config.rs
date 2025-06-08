//! OAuth2 configuration types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OAuth2 provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ProviderConfig {
    pub provider_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    /// Additional parameters to include in authorization request
    pub auth_params: HashMap<String, String>,
    /// Whether to use PKCE (recommended for public clients)
    pub use_pkce: bool,
    /// Custom user info mapping
    pub user_info_mapping: Option<UserInfoMapping>,
}

/// Mapping configuration for user info fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoMapping {
    pub subject_field: Option<String>,
    pub email_field: Option<String>,
    pub name_field: Option<String>,
    pub picture_field: Option<String>,
}

impl Default for UserInfoMapping {
    fn default() -> Self {
        Self {
            subject_field: Some("sub".to_string()),
            email_field: Some("email".to_string()),
            name_field: Some("name".to_string()),
            picture_field: Some("picture".to_string()),
        }
    }
}

/// OAuth2 client configuration
#[derive(Debug, Clone)]
pub struct OAuth2Config {
    pub providers: HashMap<String, OAuth2ProviderConfig>,
    pub state_ttl_seconds: u64,
    pub http_timeout_seconds: u64,
}

impl Default for OAuth2Config {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            state_ttl_seconds: 600, // 10 minutes
            http_timeout_seconds: 30,
        }
    }
}

impl OAuth2Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_provider(mut self, config: OAuth2ProviderConfig) -> Self {
        self.providers.insert(config.provider_id.clone(), config);
        self
    }

    pub fn with_state_ttl(mut self, seconds: u64) -> Self {
        self.state_ttl_seconds = seconds;
        self
    }

    pub fn with_http_timeout(mut self, seconds: u64) -> Self {
        self.http_timeout_seconds = seconds;
        self
    }
}
