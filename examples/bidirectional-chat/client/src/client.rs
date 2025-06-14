use anyhow::{Context, Result};
use bidirectional_chat_api::{
    ChatServiceClient, JoinRoomRequest, ListRoomsRequest,
    MessageReceivedNotification, SendMessageRequest,
    SystemAnnouncementNotification, UserJoinedNotification, UserKickedNotification,
    UserLeftNotification,
};
use ras_jsonrpc_bidirectional_client::ClientBuilder;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::ui::state::{AppState, ConnectionStatus};

pub struct ChatClient {
    client: Arc<ChatServiceClient>,
    app_state: Arc<Mutex<AppState>>,
}

impl Clone for ChatClient {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            app_state: Arc::clone(&self.app_state),
        }
    }
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

        // Build the WebSocket client with JWT authentication
        let ws_client = ClientBuilder::new(websocket_url)
            .with_jwt_token(token)
            .with_jwt_in_header(true)
            .with_heartbeat_interval(Some(std::time::Duration::from_secs(30)))
            .with_reconnect_config(ras_jsonrpc_bidirectional_client::ReconnectConfig {
                enabled: true,
                max_attempts: 10,
                initial_delay: std::time::Duration::from_secs(1),
                max_delay: std::time::Duration::from_secs(30),
                backoff_multiplier: 2.0,
                jitter: 0.1,
            })
            .build()
            .await
            .context("Failed to build WebSocket client")?;

        // Connect to the server
        ws_client
            .connect()
            .await
            .context("Failed to connect to chat server")?;

        info!("Successfully connected to chat server");

        // Update connection status
        {
            let mut state = app_state.lock().await;
            state.set_connection_status(ConnectionStatus::Connected);
        }

        // Create the ChatService client wrapper
        let mut service_client = ChatServiceClient::new(ws_client);

        // Set up notification handlers
        Self::setup_notification_handlers(&mut service_client, app_state.clone());

        let client = Arc::new(service_client);

        Ok(Self { client, app_state })
    }

    fn setup_notification_handlers(client: &mut ChatServiceClient, app_state: Arc<Mutex<AppState>>) {
        // Handle incoming messages
        let message_state = app_state.clone();
        client.on_message_received(move |notification: MessageReceivedNotification| {
            let state = message_state.clone();
            tokio::spawn(async move {
                let mut app = state.lock().await;
                app.add_incoming_message(notification.username, notification.text);
            });
        });

        // Handle user joined
        let joined_state = app_state.clone();
        client.on_user_joined(move |notification: UserJoinedNotification| {
            let state = joined_state.clone();
            tokio::spawn(async move {
                let mut app = state.lock().await;
                app.add_user(notification.username.clone());
                app.add_system_message(format!("{} joined the room", notification.username));
            });
        });

        // Handle user left
        let left_state = app_state.clone();
        client.on_user_left(move |notification: UserLeftNotification| {
            let state = left_state.clone();
            tokio::spawn(async move {
                let mut app = state.lock().await;
                app.remove_user(&notification.username);
                app.add_system_message(format!("{} left the room", notification.username));
            });
        });

        // Handle system announcements
        let announce_state = app_state.clone();
        client.on_system_announcement(move |notification: SystemAnnouncementNotification| {
            let state = announce_state.clone();
            tokio::spawn(async move {
                let mut app = state.lock().await;
                app.add_system_message(format!("📢 {}", notification.message));
            });
        });

        // Handle user kicked
        let kicked_state = app_state.clone();
        client.on_user_kicked(move |notification: UserKickedNotification| {
            let state = kicked_state.clone();
            tokio::spawn(async move {
                let mut app = state.lock().await;
                app.remove_user(&notification.username);
                app.add_system_message(format!(
                    "{} was kicked (reason: {})",
                    notification.username,
                    notification.reason
                ));
            });
        });
    }


    pub async fn send_message(&self, message: String) -> Result<()> {
        let request = SendMessageRequest { text: message };

        match self.client.send_message(request).await {
            Ok(response) => {
                debug!("Message sent successfully: {:?}", response);
                Ok(())
            }
            Err(e) => {
                error!("Failed to send message: {}", e);
                Err(anyhow::anyhow!("Failed to send message: {}", e))
            }
        }
    }

    pub async fn join_room(&self, room_name: String) -> Result<()> {
        let request = JoinRoomRequest { room_name };

        match self.client.join_room(request).await {
            Ok(_response) => {
                info!("Successfully joined room");
                Ok(())
            }
            Err(e) => {
                error!("Failed to join room: {}", e);
                Err(anyhow::anyhow!("Failed to join room: {}", e))
            }
        }
    }

    pub async fn _list_rooms(&self) -> Result<Vec<String>> {
        let request = ListRoomsRequest {};

        match self.client.list_rooms(request).await {
            Ok(response) => Ok(response
                .rooms
                .into_iter()
                .map(|room| room.room_name)
                .collect()),
            Err(e) => {
                error!("Failed to list rooms: {}", e);
                Err(anyhow::anyhow!("Failed to list rooms: {}", e))
            }
        }
    }

    pub async fn handle_pending_messages(&self) {
        loop {
            // Check for pending messages every 100ms
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let message = {
                let mut state = self.app_state.lock().await;
                state.get_next_pending_message()
            };

            if let Some(message) = message {
                if let Err(e) = self.send_message(message).await {
                    let mut state = self.app_state.lock().await;
                    state.add_system_message(format!("Failed to send: {}", e));
                }
            }
        }
    }
}