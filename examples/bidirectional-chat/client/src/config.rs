use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub websocket_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".to_string(),
            websocket_url: "ws://localhost:3000/ws".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            toml::from_str(&contents)
                .context("Failed to parse config file")
        } else {
            // Create default config
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_dir = config_path.parent().unwrap();
        
        std::fs::create_dir_all(config_dir)
            .context("Failed to create config directory")?;
            
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
            
        std::fs::write(&config_path, contents)
            .context("Failed to write config file")?;
            
        Ok(())
    }
    
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::config_dir()
            .context("Failed to get config directory")?;
        Ok(home.join("bidirectional-chat-client"))
    }
    
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }
    
    pub fn token_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("token.json"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredToken {
    pub token: String,
    pub username: String,
}

impl StoredToken {
    pub fn load() -> Result<Option<Self>> {
        let token_path = Config::token_path()?;
        
        if !token_path.exists() {
            return Ok(None);
        }
        
        let contents = std::fs::read_to_string(&token_path)
            .context("Failed to read token file")?;
            
        let token: StoredToken = serde_json::from_str(&contents)
            .context("Failed to parse token file")?;
            
        Ok(Some(token))
    }
    
    pub fn save(&self) -> Result<()> {
        let token_path = Config::token_path()?;
        let token_dir = token_path.parent().unwrap();
        
        std::fs::create_dir_all(token_dir)
            .context("Failed to create token directory")?;
            
        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize token")?;
            
        std::fs::write(&token_path, contents)
            .context("Failed to write token file")?;
            
        Ok(())
    }
    
    pub fn delete() -> Result<()> {
        let token_path = Config::token_path()?;
        
        if token_path.exists() {
            std::fs::remove_file(&token_path)
                .context("Failed to delete token file")?;
        }
        
        Ok(())
    }
}