use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_at: i64,
    pub user_id: String,
}

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

    pub async fn login(&self, username: String, password: String) -> Result<AuthResponse> {
        let response = self
            .client
            .post(&format!("{}/auth/login", self.base_url))
            .json(&LoginRequest { username, password })
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Login failed: {}", error_text)
        }
    }

    pub async fn register(&self, username: String, password: String) -> Result<AuthResponse> {
        let response = self
            .client
            .post(&format!("{}/auth/register", self.base_url))
            .json(&RegisterRequest { username, password })
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