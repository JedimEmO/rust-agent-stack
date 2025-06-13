use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use ras_jsonrpc_bidirectional_client::{Client, ConnectionHandle};
use ras_jsonrpc_types::{Id, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::ui::state::{AppState, ConnectionStatus};

// Message types matching the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReceivedNotification {
    pub message_id: String,
    pub username: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJoinedNotification {
    pub username: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLeftNotification {
    pub username: String,
    pub timestamp: String,
}

pub struct ChatClient {
    client: Client,
    connection: ConnectionHandle,
    app_state: Arc<Mutex<AppState>>,
}

impl ChatClient {
    pub async fn new(
        websocket_url: String,
        token: String,
        app_state: Arc<Mutex<AppState>>,
    ) -> Result<Self> {
        info!("Connecting to chat server at: {}", websocket_url);
        
        // Update connection status
        {
            let mut state = app_state.lock().await;
            state.set_connection_status(ConnectionStatus::Connecting);
        }
        
        // Create the client with authorization header
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().context("Invalid token format")?,
        );
        
        let (client, connection) = Client::connect_with_headers(&websocket_url, headers)
            .await
            .context("Failed to connect to chat server")?;
        
        info!("Successfully connected to chat server");
        
        // Update connection status
        {
            let mut state = app_state.lock().await;
            state.set_connection_status(ConnectionStatus::Connected);
        }
        
        // Start notification handler
        let notification_state = app_state.clone();
        tokio::spawn(async move {
            handle_notifications(connection.clone(), notification_state).await;
        });
        
        // Start message sender task
        let sender_client = client.clone();
        let sender_state = app_state.clone();
        tokio::spawn(async move {
            handle_outgoing_messages(sender_client, sender_state).await;
        });
        
        Ok(Self {
            client,
            connection,
            app_state,
        })
    }
    
    pub async fn send_message(&self, message: String) -> Result<()> {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "send_message".to_string(),
            params: Some(json!(SendMessageRequest { message })),
            id: Some(Id::Number(1)),
        };
        
        self.client
            .send_request(request)
            .await
            .context("Failed to send message")?;
            
        Ok(())
    }
}

async fn handle_notifications(
    mut connection: ConnectionHandle,
    app_state: Arc<Mutex<AppState>>,
) {
    while let Some(notification) = connection.notifications.recv().await {
        debug!("Received notification: {}", notification.method);
        
        match notification.method.as_str() {
            "message_received" => {
                if let Some(params) = notification.params {
                    match serde_json::from_value::<MessageReceivedNotification>(params) {
                        Ok(msg) => {
                            let mut state = app_state.lock().await;
                            state.add_incoming_message(msg.username, msg.message);
                        }
                        Err(e) => {
                            error!("Failed to parse message_received notification: {}", e);
                        }
                    }
                }
            }
            "user_joined" => {
                if let Some(params) = notification.params {
                    match serde_json::from_value::<UserJoinedNotification>(params) {
                        Ok(user) => {
                            let mut state = app_state.lock().await;
                            state.add_user(user.username);
                        }
                        Err(e) => {
                            error!("Failed to parse user_joined notification: {}", e);
                        }
                    }
                }
            }
            "user_left" => {
                if let Some(params) = notification.params {
                    match serde_json::from_value::<UserLeftNotification>(params) {
                        Ok(user) => {
                            let mut state = app_state.lock().await;
                            state.remove_user(&user.username);
                        }
                        Err(e) => {
                            error!("Failed to parse user_left notification: {}", e);
                        }
                    }
                }
            }
            _ => {
                warn!("Received unknown notification: {}", notification.method);
            }
        }
    }
    
    // Connection closed
    let mut state = app_state.lock().await;
    state.set_connection_status(ConnectionStatus::Disconnected);
}

async fn handle_outgoing_messages(
    client: Client,
    app_state: Arc<Mutex<AppState>>,
) {
    let mut counter = 1u64;
    
    loop {
        // Check for pending messages every 100ms
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let message = {
            let mut state = app_state.lock().await;
            state.get_next_pending_message()
        };
        
        if let Some(message) = message {
            let request = Request {
                jsonrpc: "2.0".to_string(),
                method: "send_message".to_string(),
                params: Some(json!(SendMessageRequest { message: message.clone() })),
                id: Some(Id::Number(counter)),
            };
            counter += 1;
            
            match client.send_request(request).await {
                Ok(_) => {
                    debug!("Message sent successfully");
                }
                Err(e) => {
                    error!("Failed to send message: {}", e);
                    let mut state = app_state.lock().await;
                    state.add_system_message(format!("Failed to send message: {}", e));
                }
            }
        }
    }
}