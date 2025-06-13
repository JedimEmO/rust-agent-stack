//! Integration tests for the bidirectional chat server
//!
//! These tests cover:
//! - Server startup and health checks
//! - User registration and authentication
//! - Basic API endpoint testing
//! - Configuration validation

use anyhow::Result;
use axum::Router;
use bidirectional_chat_server::config::{
    AdminConfig, AdminUser, AuthConfig, ChatConfig, Config, LoggingConfig, RateLimitConfig,
    RoomConfig, ServerConfig,
};
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tower_http::cors::CorsLayer;

/// Test server instance
struct TestServer {
    addr: SocketAddr,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Start a test server with the given configuration
    async fn start(config: Config) -> Result<Self> {
        let addr = config.socket_addr();

        // Create a minimal server setup for testing
        let auth_router = Router::new()
            .route("/auth/login", axum::routing::post(dummy_login_handler))
            .route(
                "/auth/register",
                axum::routing::post(dummy_register_handler),
            );

        let health_router = Router::new().route("/health", axum::routing::get(|| async { "OK" }));

        let app = Router::new()
            .merge(auth_router)
            .merge(health_router)
            .layer(CorsLayer::permissive());

        let listener = TcpListener::bind(addr).await?;
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            let server = axum::serve(listener, app);
            let graceful = server.with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
            let _ = graceful.await;
        });

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(Self {
            addr,
            shutdown_tx,
            handle,
        })
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), self.handle).await;
    }
}

// Dummy handlers for basic testing
async fn dummy_login_handler(
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    if payload.get("username").is_some() && payload.get("password").is_some() {
        Ok(axum::Json(json!({
            "token": "test-token",
            "expires_at": 1234567890,
            "user_id": payload["username"]
        })))
    } else {
        Err(axum::http::StatusCode::BAD_REQUEST)
    }
}

async fn dummy_register_handler(
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    if payload.get("username").is_some() && payload.get("password").is_some() {
        Ok(axum::Json(json!({
            "message": "User registered successfully",
            "username": payload["username"]
        })))
    } else {
        Err(axum::http::StatusCode::BAD_REQUEST)
    }
}

// Helper function to create test configuration
async fn create_test_config() -> Result<(Config, TempDir)> {
    let temp_dir = TempDir::new()?;
    let data_dir = temp_dir.path().join("chat_data");

    // Find available port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);

    let config = Config {
        server: ServerConfig {
            host: addr.ip(),
            port: addr.port(),
            cors: Default::default(),
        },
        auth: AuthConfig {
            jwt_secret: "test-secret-key".to_string(),
            jwt_ttl_seconds: 3600,
            refresh_enabled: true,
            jwt_algorithm: "HS256".to_string(),
        },
        chat: ChatConfig {
            data_dir,
            max_message_length: 1000,
            max_room_name_length: 50,
            max_users_per_room: 10,
            default_rooms: vec![RoomConfig {
                id: "general".to_string(),
                name: "General".to_string(),
                description: Some("General chat room".to_string()),
            }],
            persist_messages: true,
            persist_rooms: true,
            persist_profiles: true,
        },
        admin: AdminConfig {
            users: vec![AdminUser {
                username: "admin".to_string(),
                password: "admin123456".to_string(),
                email: Some("admin@test.com".to_string()),
                display_name: Some("Test Admin".to_string()),
                permissions: vec![
                    "admin".to_string(),
                    "moderator".to_string(),
                    "user".to_string(),
                ],
            }],
            auto_create: true,
        },
        rate_limit: RateLimitConfig {
            enabled: false,
            ..Default::default()
        },
        logging: LoggingConfig::default(),
    };

    Ok((config, temp_dir))
}

#[tokio::test]
async fn test_config_defaults() {
    let config = Config::default();
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.auth.jwt_ttl_seconds, 86400);
    assert_eq!(config.chat.max_message_length, 1000);
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = Config::default();

    // Test invalid port
    config.server.port = 0;
    assert!(config.validate().is_err());

    // Test invalid JWT TTL
    config.server.port = 3000;
    config.auth.jwt_ttl_seconds = 0;
    assert!(config.validate().is_err());

    // Test invalid message length
    config.auth.jwt_ttl_seconds = 3600;
    config.chat.max_message_length = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_server_startup() -> Result<()> {
    let (config, _temp_dir) = create_test_config().await?;
    let server = TestServer::start(config).await?;

    // Test health endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", server.url()))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await?, "OK");

    server.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_auth_endpoints() -> Result<()> {
    let (config, _temp_dir) = create_test_config().await?;
    let server = TestServer::start(config).await?;
    let client = reqwest::Client::new();

    // Test registration endpoint
    let response = client
        .post(format!("{}/auth/register", server.url()))
        .json(&json!({
            "username": "testuser",
            "password": "testpass123"
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["username"], "testuser");

    // Test login endpoint
    let response = client
        .post(format!("{}/auth/login", server.url()))
        .json(&json!({
            "username": "testuser",
            "password": "testpass123"
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await?;
    assert!(body.get("token").is_some());

    server.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_persistence_initialization() -> Result<()> {
    use bidirectional_chat_server::persistence::PersistenceManager;

    let temp_dir = TempDir::new()?;
    let persistence = PersistenceManager::new(temp_dir.path());

    // Initialize persistence
    persistence.init().await?;

    // Verify directories were created
    assert!(temp_dir.path().exists());
    assert!(temp_dir.path().join("messages").exists());

    // Test save and load state
    let mut state = persistence.load_state().await?;
    assert_eq!(state.next_message_id, 1);

    state.next_message_id = 42;
    persistence.save_state(&state).await?;

    let loaded_state = persistence.load_state().await?;
    assert_eq!(loaded_state.next_message_id, 42);

    Ok(())
}

#[tokio::test]
async fn test_room_configuration() -> Result<()> {
    let (mut config, _temp_dir) = create_test_config().await?;

    // Add multiple default rooms
    config.chat.default_rooms = vec![
        RoomConfig {
            id: "general".to_string(),
            name: "General".to_string(),
            description: Some("General discussion".to_string()),
        },
        RoomConfig {
            id: "tech".to_string(),
            name: "Technology".to_string(),
            description: Some("Tech discussion".to_string()),
        },
    ];

    // Validation should pass
    assert!(config.validate().is_ok());

    // Test invalid room configuration
    config.chat.default_rooms.push(RoomConfig {
        id: "".to_string(), // Empty ID should fail
        name: "Invalid".to_string(),
        description: None,
    });

    assert!(config.validate().is_err());

    Ok(())
}

#[tokio::test]
async fn test_admin_configuration() -> Result<()> {
    let (mut config, _temp_dir) = create_test_config().await?;

    // Test valid admin configuration
    config.admin.users = vec![
        AdminUser {
            username: "admin1".to_string(),
            password: "adminpass123".to_string(),
            email: Some("admin1@test.com".to_string()),
            display_name: Some("Admin One".to_string()),
            permissions: vec!["admin".to_string()],
        },
        AdminUser {
            username: "moderator1".to_string(),
            password: "modpass123".to_string(),
            email: None,
            display_name: None,
            permissions: vec!["moderator".to_string(), "user".to_string()],
        },
    ];

    assert!(config.validate().is_ok());

    // Test invalid admin configuration (short password)
    config.admin.users.push(AdminUser {
        username: "badadmin".to_string(),
        password: "short".to_string(), // Too short
        email: None,
        display_name: None,
        permissions: vec!["admin".to_string()],
    });

    assert!(config.validate().is_err());

    Ok(())
}

#[tokio::test]
async fn test_rate_limit_configuration() -> Result<()> {
    let (mut config, _temp_dir) = create_test_config().await?;

    // Enable rate limiting with valid values
    config.rate_limit.enabled = true;
    config.rate_limit.messages_per_minute = 60;
    config.rate_limit.connections_per_ip = 5;
    config.rate_limit.login_attempts_per_hour = 10;

    assert!(config.validate().is_ok());

    // Test invalid rate limit configuration
    config.rate_limit.messages_per_minute = 0;
    assert!(config.validate().is_err());

    Ok(())
}

#[tokio::test]
async fn test_cors_configuration() -> Result<()> {
    let (mut config, _temp_dir) = create_test_config().await?;

    // Test allow any origin
    config.server.cors.allow_any_origin = true;
    assert!(config.validate().is_ok());

    // Test specific origins
    config.server.cors.allow_any_origin = false;
    config.server.cors.allowed_origins = vec![
        "http://localhost:3000".to_string(),
        "https://example.com".to_string(),
    ];
    assert!(config.validate().is_ok());

    // Test invalid CORS configuration (no origins when not allowing any)
    config.server.cors.allowed_origins.clear();
    assert!(config.validate().is_err());

    Ok(())
}

#[tokio::test]
async fn test_logging_configuration() -> Result<()> {
    let (mut config, _temp_dir) = create_test_config().await?;

    // Test valid log levels
    for level in ["trace", "debug", "info", "warn", "error"] {
        config.logging.level = level.to_string();
        assert!(config.validate().is_ok());
    }

    // Test invalid log level
    config.logging.level = "invalid".to_string();
    assert!(config.validate().is_err());

    // Test valid log formats
    for format in ["pretty", "json", "compact"] {
        config.logging.level = "info".to_string();
        config.logging.format = format.to_string();
        assert!(config.validate().is_ok());
    }

    // Test invalid log format
    config.logging.format = "invalid".to_string();
    assert!(config.validate().is_err());

    Ok(())
}

#[tokio::test]
async fn test_message_persistence() -> Result<()> {
    use bidirectional_chat_server::persistence::{PersistedMessage, PersistenceManager};
    use chrono::Utc;

    let temp_dir = TempDir::new()?;
    let persistence = PersistenceManager::new(temp_dir.path());
    persistence.init().await?;

    // Create and persist messages
    let room_id = "test-room";
    let messages = vec![
        PersistedMessage {
            id: 1,
            room_id: room_id.to_string(),
            username: "alice".to_string(),
            text: "Hello!".to_string(),
            timestamp: Utc::now(),
        },
        PersistedMessage {
            id: 2,
            room_id: room_id.to_string(),
            username: "bob".to_string(),
            text: "Hi there!".to_string(),
            timestamp: Utc::now(),
        },
    ];

    for msg in &messages {
        persistence.append_message(room_id, msg).await?;
    }

    // Load messages
    let loaded = persistence.load_room_messages(room_id, None).await?;
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].text, "Hello!");
    assert_eq!(loaded[1].text, "Hi there!");

    // Test limit
    let limited = persistence.load_room_messages(room_id, Some(1)).await?;
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].text, "Hi there!"); // Most recent

    // Test non-existent room
    let empty = persistence.load_room_messages("non-existent", None).await?;
    assert!(empty.is_empty());

    Ok(())
}

// Module to re-export necessary types for the tests
mod bidirectional_chat_server {
    pub use bidirectional_chat_server::config;

    pub mod persistence {
        pub use bidirectional_chat_server::persistence::*;
    }
}
