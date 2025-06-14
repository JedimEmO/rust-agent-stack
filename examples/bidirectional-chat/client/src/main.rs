use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod ui;
mod client;
mod config;
mod auth;

use crate::config::Config;

#[derive(Parser)]
#[command(name = "bidirectional-chat-client")]
#[command(about = "A terminal chat client with animated ASCII cat avatars", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Server URL (overrides config file)
    #[arg(long, env = "CHAT_SERVER_URL")]
    server_url: Option<String>,
    
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a new user account
    Register {
        /// Username for the new account
        #[arg(short, long)]
        username: String,
    },
    
    /// Login to an existing account
    Login {
        /// Username to login with
        #[arg(short, long)]
        username: String,
    },
    
    /// Start the interactive chat interface
    Chat,
    
    /// Logout and clear stored credentials
    Logout,
    
    /// Display configuration information
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let filter = if cli.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env()
            .add_directive("bidirectional_chat_client=info".parse()?)
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    // Load configuration
    let mut config = Config::load()?;
    if let Some(url) = cli.server_url {
        config.server_url = url;
    }
    
    match cli.command {
        Commands::Register { username } => {
            auth::register(&config, &username).await?;
        }
        Commands::Login { username } => {
            auth::login(&config, &username).await?;
        }
        Commands::Chat => {
            ui::run_chat_ui(&config).await?;
        }
        Commands::Logout => {
            auth::logout(&config)?;
        }
        Commands::Config => {
            println!("Current configuration:");
            println!("Server URL: {}", config.server_url);
            println!("Config file: {}", Config::config_path()?.display());
            println!("Token file: {}", Config::token_path()?.display());
        }
    }
    
    Ok(())
}