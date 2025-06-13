//! Bidirectional chat client example
//!
//! This client demonstrates:
//! - Connecting to the chat server via WebSocket
//! - Authentication with username/password
//! - Sending and receiving messages in real-time
//! - Managing chat rooms
//! - Interactive terminal UI

use anyhow::Result;
use bidirectional_chat_api::*;
use chrono::Local;
use clap::{Parser, Subcommand};
use console::{Style, Term, style};
use dialoguer::{Input, Password, Select, theme::ColorfulTheme};
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::RwLock;
use tracing::error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL
    #[arg(short, long, default_value = "http://localhost:3000")]
    server: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a new user
    Register {
        /// Username
        #[arg(short, long)]
        username: Option<String>,
    },
    /// Login with existing credentials
    Login {
        /// Username
        #[arg(short, long)]
        username: Option<String>,
    },
    /// Start interactive chat session
    Chat,
}

// Chat client state
struct ChatClient {
    client: ChatServiceClient,
    current_room: Arc<RwLock<Option<String>>>,
    username: Arc<RwLock<Option<String>>>,
    running: Arc<AtomicBool>,
    term: Term,
}

impl ChatClient {
    async fn new(server_url: &str, token: String) -> Result<Self> {
        let ws_url = server_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let ws_url = format!("{}/ws", ws_url);

        let client = ChatServiceClientBuilder::new(ws_url)
            .with_jwt_token(token)
            .with_request_timeout(Duration::from_secs(30))
            .build()
            .await?;

        Ok(Self {
            client,
            current_room: Arc::new(RwLock::new(None)),
            username: Arc::new(RwLock::new(None)),
            running: Arc::new(AtomicBool::new(true)),
            term: Term::stdout(),
        })
    }

    async fn setup_handlers(&mut self) {
        let term = self.term.clone();
        let current_room = self.current_room.clone();

        // Message received handler
        self.client.on_message_received({
            let term = term.clone();
            let current_room = current_room.clone();
            move |notification| {
                let term = term.clone();
                let current_room = current_room.clone();
                tokio::spawn(async move {
                    let room = current_room.read().await;
                    if room.as_ref() == Some(&notification.room_id) {
                        let time = Local::now().format("%H:%M:%S");
                        let _ = term.write_line(&format!(
                            "{} {} {}: {}",
                            style(format!("[{}]", time)).dim(),
                            style(&notification.username).cyan().bold(),
                            style("says").dim(),
                            notification.text
                        ));
                    }
                });
            }
        });

        // User joined handler
        self.client.on_user_joined({
            let term = term.clone();
            let current_room = current_room.clone();
            move |notification| {
                let term = term.clone();
                let current_room = current_room.clone();
                tokio::spawn(async move {
                    let room = current_room.read().await;
                    if room.as_ref() == Some(&notification.room_id) {
                        let _ = term.write_line(&format!(
                            "{} {} joined the room (total: {} users)",
                            style("→").green(),
                            style(&notification.username).green().bold(),
                            notification.user_count
                        ));
                    }
                });
            }
        });

        // User left handler
        self.client.on_user_left({
            let term = term.clone();
            let current_room = current_room.clone();
            move |notification| {
                let term = term.clone();
                let current_room = current_room.clone();
                tokio::spawn(async move {
                    let room = current_room.read().await;
                    if room.as_ref() == Some(&notification.room_id) {
                        let _ = term.write_line(&format!(
                            "{} {} left the room (total: {} users)",
                            style("←").red(),
                            style(&notification.username).red(),
                            notification.user_count
                        ));
                    }
                });
            }
        });

        // System announcement handler
        self.client.on_system_announcement({
            let term = term.clone();
            move |notification| {
                let term = term.clone();
                tokio::spawn(async move {
                    let style = match notification.level {
                        AnnouncementLevel::Info => Style::new().blue(),
                        AnnouncementLevel::Warning => Style::new().yellow(),
                        AnnouncementLevel::Error => Style::new().red().bold(),
                    };
                    let _ = term.write_line(&format!(
                        "{} {}",
                        style.apply_to("📢 System:"),
                        notification.message
                    ));
                });
            }
        });

        // User kicked handler
        self.client.on_user_kicked({
            let term = term.clone();
            let running = self.running.clone();
            move |notification| {
                let term = term.clone();
                let running = running.clone();
                tokio::spawn(async move {
                    let _ = term.write_line(&format!(
                        "{} You have been kicked: {}",
                        style("⚠️").red().bold(),
                        style(&notification.reason).red()
                    ));
                    running.store(false, Ordering::SeqCst);
                });
            }
        });

        // Room created handler
        self.client.on_room_created({
            let term = term.clone();
            move |notification| {
                let term = term.clone();
                tokio::spawn(async move {
                    let _ = term.write_line(&format!(
                        "{} New room created: {} ({})",
                        style("🏠").green(),
                        style(&notification.room_info.room_name).green().bold(),
                        style(&notification.room_info.room_id).dim()
                    ));
                });
            }
        });
    }

    async fn connect(&mut self) -> Result<()> {
        self.client.connect().await?;
        self.setup_handlers().await;
        Ok(())
    }

    async fn run_interactive(&mut self, username: String) -> Result<()> {
        *self.username.write().await = Some(username.clone());

        self.term.clear_screen()?;
        println!("{}", style("=== Chat Client ===").bold().cyan());
        println!("Welcome, {}!", style(&username).green().bold());
        println!();

        // Auto-join general room
        match self
            .client
            .join_room(JoinRoomRequest {
                room_name: "general".to_string(),
            })
            .await
        {
            Ok(response) => {
                *self.current_room.write().await = Some(response.room_id.clone());
                println!(
                    "Joined room '{}' ({} users online)",
                    style("general").cyan().bold(),
                    response.user_count
                );
            }
            Err(e) => {
                error!("Failed to join general room: {}", e);
            }
        }

        println!();
        println!("Commands:");
        println!("  /rooms     - List all rooms");
        println!("  /join <room> - Join a room");
        println!("  /leave     - Leave current room");
        println!("  /help      - Show this help");
        println!("  /quit      - Exit chat");
        println!();

        // Message input loop
        while self.running.load(Ordering::SeqCst) {
            let input: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(">")
                .allow_empty(true)
                .interact_text()?;

            if input.trim().is_empty() {
                continue;
            }

            if input.starts_with('/') {
                self.handle_command(&input).await?;
            } else {
                // Send message
                if self.current_room.read().await.is_some() {
                    match self
                        .client
                        .send_message(SendMessageRequest { text: input })
                        .await
                    {
                        Ok(_) => {
                            // Message sent successfully
                        }
                        Err(e) => {
                            self.term.write_line(&format!(
                                "{} Failed to send message: {}",
                                style("❌").red(),
                                e
                            ))?;
                        }
                    }
                } else {
                    self.term.write_line(&format!(
                        "{} You must join a room first! Use /join <room>",
                        style("⚠️").yellow()
                    ))?;
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        match parts.get(0).map(|s| s.to_lowercase()).as_deref() {
            Some("/rooms") => self.list_rooms().await?,
            Some("/join") => {
                if let Some(room_name) = parts.get(1) {
                    self.join_room(room_name).await?;
                } else {
                    self.term.write_line("Usage: /join <room_name>")?;
                }
            }
            Some("/leave") => self.leave_room().await?,
            Some("/help") => self.show_help()?,
            Some("/quit") => {
                self.running.store(false, Ordering::SeqCst);
                self.term.write_line("Goodbye!")?;
            }
            _ => {
                self.term
                    .write_line(&format!("Unknown command: {}", command))?;
            }
        }

        Ok(())
    }

    async fn list_rooms(&mut self) -> Result<()> {
        match self.client.list_rooms(ListRoomsRequest {}).await {
            Ok(response) => {
                self.term.write_line("\nAvailable rooms:")?;
                for room in &response.rooms {
                    let marker = if self.current_room.read().await.as_ref() == Some(&room.room_id) {
                        style("*").green().bold()
                    } else {
                        style(" ")
                    };
                    self.term.write_line(&format!(
                        " {} {} ({} users) - ID: {}",
                        marker,
                        style(&room.room_name).cyan(),
                        room.user_count,
                        style(&room.room_id).dim()
                    ))?;
                }
                self.term.write_line("")?;
            }
            Err(e) => {
                self.term.write_line(&format!(
                    "{} Failed to list rooms: {}",
                    style("❌").red(),
                    e
                ))?;
            }
        }
        Ok(())
    }

    async fn join_room(&mut self, room_name: &str) -> Result<()> {
        // Leave current room first if in one
        if self.current_room.read().await.is_some() {
            self.leave_room().await?;
        }

        match self
            .client
            .join_room(JoinRoomRequest {
                room_name: room_name.to_string(),
            })
            .await
        {
            Ok(response) => {
                *self.current_room.write().await = Some(response.room_id.clone());
                self.term.write_line(&format!(
                    "{} Joined room '{}' ({} users)",
                    style("✓").green(),
                    style(room_name).cyan().bold(),
                    response.user_count
                ))?;
            }
            Err(e) => {
                self.term.write_line(&format!(
                    "{} Failed to join room: {}",
                    style("❌").red(),
                    e
                ))?;
            }
        }
        Ok(())
    }

    async fn leave_room(&mut self) -> Result<()> {
        if let Some(room_id) = self.current_room.read().await.clone() {
            match self
                .client
                .leave_room(LeaveRoomRequest {
                    room_id: room_id.clone(),
                })
                .await
            {
                Ok(_) => {
                    *self.current_room.write().await = None;
                    self.term
                        .write_line(&format!("{} Left the room", style("✓").green()))?;
                }
                Err(e) => {
                    self.term.write_line(&format!(
                        "{} Failed to leave room: {}",
                        style("❌").red(),
                        e
                    ))?;
                }
            }
        } else {
            self.term.write_line("You're not in any room")?;
        }
        Ok(())
    }

    fn show_help(&self) -> Result<()> {
        self.term.write_line("\nAvailable commands:")?;
        self.term
            .write_line("  /rooms       - List all available rooms")?;
        self.term
            .write_line("  /join <room> - Join a specific room")?;
        self.term
            .write_line("  /leave       - Leave the current room")?;
        self.term
            .write_line("  /help        - Show this help message")?;
        self.term
            .write_line("  /quit        - Exit the chat client")?;
        self.term.write_line("")?;
        Ok(())
    }
}

async fn register_user(server_url: &str, username: String) -> Result<()> {
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .interact()?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/register", server_url))
        .json(&json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        println!("{} User registered successfully!", style("✓").green());
    } else {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("Registration failed: {}", error));
    }

    Ok(())
}

async fn login_user(server_url: &str, username: String) -> Result<String> {
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .interact()?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/login", server_url))
        .json(&json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        if let Some(token) = data["token"].as_str() {
            println!("{} Login successful!", style("✓").green());
            return Ok(token.to_string());
        }
    }

    Err(anyhow::anyhow!("Login failed"))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_ansi(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bidirectional_chat_client=debug".parse()?)
                .add_directive("ras_jsonrpc_bidirectional_client=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Register { username } => {
            let username = username.unwrap_or_else(|| {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Username")
                    .interact_text()
                    .unwrap()
            });

            register_user(&cli.server, username).await?;
        }
        Commands::Login { username } => {
            let username = username.unwrap_or_else(|| {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Username")
                    .interact_text()
                    .unwrap()
            });

            let token = login_user(&cli.server, username.clone()).await?;
            println!("\nToken: {}", style(&token).dim());
            println!("\nYou can now start the chat client with:");
            println!(
                "  {} chat",
                style("cargo run -p bidirectional-chat-client").cyan()
            );
        }
        Commands::Chat => {
            // Interactive mode - ask for credentials
            let choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Do you want to login or use an existing token?")
                .items(&["Login with username/password", "Use existing token"])
                .default(0)
                .interact()?;

            let (token, username) = match choice {
                0 => {
                    // Login flow
                    let username: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Username")
                        .interact_text()?;

                    let token = login_user(&cli.server, username.clone()).await?;
                    (token, username)
                }
                1 => {
                    // Token flow
                    let token: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("JWT Token")
                        .interact_text()?;

                    // Extract username from token (in real app, would decode JWT)
                    let username: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Username")
                        .interact_text()?;

                    (token, username)
                }
                _ => unreachable!(),
            };

            // Create and run chat client
            let mut client = ChatClient::new(&cli.server, token).await?;
            client.connect().await?;
            client.run_interactive(username).await?;

            // Disconnect
            client.client.disconnect().await?;
        }
    }

    Ok(())
}
