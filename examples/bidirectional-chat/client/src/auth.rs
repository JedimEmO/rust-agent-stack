use anyhow::{anyhow, Context, Result};
use crate::config::{Config, StoredToken};
use tracing::{debug, info};
use bidirectional_chat_api::auth::{
    ChatAuthServiceClientBuilder,
    RegisterRequest, LoginRequest,
};

pub async fn register(config: &Config, username: &str) -> Result<()> {
    info!("Starting registration process for user: {}", username);
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
    
    // Create REST client
    let client = ChatAuthServiceClientBuilder::new()
        .server_url(&config.server_url)
        .build()
        .map_err(|e| anyhow!("Failed to create auth client: {}", e))?;
    
    debug!("Created auth client with server URL: {}", config.server_url);
    
    // Send registration request
    let register_request = RegisterRequest {
        username: username.to_string(),
        password,
        email: None,
        display_name: None,
    };
    
    info!("Sending registration request");
    
    let register_response = client
        .post_auth_register(register_request)
        .await
        .map_err(|e| anyhow!("Failed to register user: {}", e))?;
    
    info!("Registration completed successfully for user: {}", register_response.username);
    println!("✓ Successfully registered user: {}", register_response.username);
    println!("You can now login with: bidirectional-chat-client login -u {}", username);
    
    Ok(())
}

pub async fn login(config: &Config, username: &str) -> Result<()> {
    info!("Starting login process for user: {}", username);
    println!("Logging in as: {}", username);
    
    // Prompt for password
    let password = rpassword::prompt_password("Password: ")
        .context("Failed to read password")?;
    
    debug!("Password entered, length: {}", password.len());
    
    // Create REST client
    let client = ChatAuthServiceClientBuilder::new()
        .server_url(&config.server_url)
        .build()
        .map_err(|e| anyhow!("Failed to create auth client: {}", e))?;
    
    debug!("Created auth client with server URL: {}", config.server_url);
    
    // Send login request
    let login_request = LoginRequest {
        username: username.to_string(),
        password,
        provider: None, // Use default provider
    };
    
    info!("Sending login request");
    
    let login_response = client
        .post_auth_login(login_request)
        .await
        .map_err(|e| anyhow!("Failed to login: {}", e))?;
    
    info!("Successfully parsed login response for user: {}", login_response.user_id);
    
    // Save token
    let stored_token = StoredToken {
        token: login_response.token,
        username: username.to_string(), // Use the username we logged in with
    };
    
    debug!("Saving authentication token");
    stored_token.save().context("Failed to save authentication token")?;
    
    info!("Login completed successfully for user: {}", username);
    println!("✓ Successfully logged in as: {}", username);
    println!("Token expires at: {}", login_response.expires_at);
    println!("You can now start chatting with: bidirectional-chat-client chat");
    
    Ok(())
}

pub fn logout(_config: &Config) -> Result<()> {
    StoredToken::delete()?;
    println!("✓ Successfully logged out");
    Ok(())
}

pub fn get_current_token() -> Result<Option<StoredToken>> {
    StoredToken::load()
}