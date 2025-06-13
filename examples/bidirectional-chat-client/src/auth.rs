use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::config::{Config, StoredToken};

#[derive(Debug, Serialize)]
struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    id: String,
    username: String,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    user: UserInfo,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    id: String,
    username: String,
    permissions: Vec<String>,
}

pub async fn register(config: &Config, username: &str) -> Result<()> {
    println!("Registering new user: {}", username);
    
    // Prompt for password
    let password = rpassword::prompt_password("Password: ")
        .context("Failed to read password")?;
        
    if password.is_empty() {
        return Err(anyhow!("Password cannot be empty"));
    }
    
    // Confirm password
    let confirm = rpassword::prompt_password("Confirm password: ")
        .context("Failed to read password confirmation")?;
        
    if password != confirm {
        return Err(anyhow!("Passwords do not match"));
    }
    
    // Send registration request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/register", config.server_url))
        .json(&RegisterRequest {
            username: username.to_string(),
            password,
        })
        .send()
        .await
        .context("Failed to send registration request")?;
        
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow!("Registration failed: {}", error_text));
    }
    
    let register_response: RegisterResponse = response
        .json()
        .await
        .context("Failed to parse registration response")?;
        
    println!("✓ Successfully registered user: {}", register_response.username);
    println!("You can now login with: bidirectional-chat-client login -u {}", username);
    
    Ok(())
}

pub async fn login(config: &Config, username: &str) -> Result<()> {
    println!("Logging in as: {}", username);
    
    // Prompt for password
    let password = rpassword::prompt_password("Password: ")
        .context("Failed to read password")?;
        
    // Send login request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/login", config.server_url))
        .json(&LoginRequest {
            username: username.to_string(),
            password,
        })
        .send()
        .await
        .context("Failed to send login request")?;
        
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow!("Login failed: {}", error_text));
    }
    
    let login_response: LoginResponse = response
        .json()
        .await
        .context("Failed to parse login response")?;
        
    // Save token
    let stored_token = StoredToken {
        token: login_response.token,
        username: login_response.user.username.clone(),
    };
    stored_token.save()?;
    
    println!("✓ Successfully logged in as: {}", login_response.user.username);
    println!("Permissions: {:?}", login_response.user.permissions);
    println!("You can now start chatting with: bidirectional-chat-client chat");
    
    Ok(())
}

pub fn logout(config: &Config) -> Result<()> {
    StoredToken::delete()?;
    println!("✓ Successfully logged out");
    Ok(())
}

pub fn get_current_token() -> Result<Option<StoredToken>> {
    StoredToken::load()
}