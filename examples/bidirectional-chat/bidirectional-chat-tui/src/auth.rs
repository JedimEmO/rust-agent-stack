use anyhow::Result;
use reqwest::Client;

// Use types from the bidirectional-chat-api crate
use bidirectional_chat_api::auth::{
    LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
};

pub struct AuthClient {
    client: Client,
    base_url: String,
}

impl AuthClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn login(&self, username: String, password: String) -> Result<LoginResponse> {
        let response = self
            .client
            .post(&format!("{}/auth/login", self.base_url))
            .json(&LoginRequest { 
                username, 
                password,
                provider: None, // Use default "local" provider
            })
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Login failed: {}", error_text)
        }
    }

    pub async fn register(&self, username: String, password: String) -> Result<RegisterResponse> {
        let response = self
            .client
            .post(&format!("{}/auth/register", self.base_url))
            .json(&RegisterRequest { 
                username, 
                password,
                email: None,
                display_name: None,
            })
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Registration failed: {}", error_text)
        }
    }
}