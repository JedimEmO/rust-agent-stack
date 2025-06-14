//! Bidirectional chat server example
//!
//! This example demonstrates a real-time chat server using bidirectional JSON-RPC over WebSockets.
//! Features include:
//! - Multiple chat rooms
//! - User authentication with JWT
//! - Role-based permissions (user, moderator, admin)
//! - Real-time message broadcasting
//! - System announcements
//! - User management (kick functionality)

use anyhow::Result;
use axum::{Router, routing::get};
use bidirectional_chat_api::*;
use chrono::Utc;
use dashmap::DashMap;
use ras_auth_core::AuthenticatedUser;
use ras_identity_core::{UserPermissions, VerifiedIdentity};
use ras_identity_local::LocalUserProvider;
use ras_identity_session::{JwtAuthProvider, SessionConfig, SessionService};
use ras_jsonrpc_bidirectional_server::{
    DefaultConnectionManager, WebSocketServiceBuilder,
    service::{BuiltWebSocketService, websocket_handler},
};
use ras_jsonrpc_bidirectional_types::{ConnectionId, ConnectionManager};
use serde_json::json;
use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use tokio::sync::{RwLock, Mutex};
use tokio::time::Instant;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

pub mod config;
mod persistence;

use bidirectional_chat_api::auth::*;
use config::Config;
use persistence::{
    PersistedCatAvatar, PersistedMessage, PersistedRoom, PersistedUserProfile, PersistenceManager,
};

// Chat room state
#[derive(Debug, Clone)]
struct ChatRoom {
    id: String,
    name: String,
    users: HashSet<String>, // usernames
    created_at: chrono::DateTime<Utc>,
}

// User session state
#[derive(Debug, Clone)]
struct UserSession {
    username: String,
    connection_id: ConnectionId,
    current_room: Option<String>, // room_id
    joined_at: chrono::DateTime<Utc>,
}

// Typing state tracking
#[derive(Debug, Clone)]
struct TypingState {
    username: String,
    started_at: Instant,
}

// Chat server state
#[derive(Clone)]
struct ChatServer {
    rooms: Arc<DashMap<String, ChatRoom>>,
    user_sessions: Arc<DashMap<ConnectionId, UserSession>>,
    message_counter: Arc<RwLock<u64>>,
    persistence: Arc<PersistenceManager>,
    config: config::ChatConfig,
    typing_users: Arc<Mutex<HashMap<String, HashMap<String, TypingState>>>>, // room_id -> username -> typing state
}

impl ChatServer {
    #[instrument(skip_all, fields(data_dir = ?config.data_dir))]
    async fn new(config: config::ChatConfig) -> Result<Self> {
        info!("Initializing chat server with data directory");
        let persistence = Arc::new(PersistenceManager::new(&config.data_dir));
        persistence.init().await.map_err(|e| {
            error!("Failed to initialize persistence: {}", e);
            e
        })?;

        // Load persisted state
        debug!("Loading persisted state");
        let mut state = persistence.load_state().await.map_err(|e| {
            error!("Failed to load persisted state: {}", e);
            e
        })?;

        let server = Self {
            rooms: Arc::new(DashMap::new()),
            user_sessions: Arc::new(DashMap::new()),
            message_counter: Arc::new(RwLock::new(state.next_message_id)),
            persistence,
            config: config.clone(),
            typing_users: Arc::new(Mutex::new(HashMap::new())),
        };

        // Restore rooms
        if state.rooms.is_empty() {
            info!("No rooms found in persistence, creating default rooms");
            // Create default rooms from configuration
            for room_config in &config.default_rooms {
                let room = ChatRoom {
                    id: room_config.id.clone(),
                    name: room_config.name.clone(),
                    users: HashSet::new(),
                    created_at: Utc::now(),
                };
                server.rooms.insert(room_config.id.clone(), room.clone());

                // Persist the room
                state.rooms.insert(
                    room_config.id.clone(),
                    PersistedRoom {
                        id: room.id,
                        name: room.name,
                        created_at: room.created_at,
                        users: room.users.clone(),
                    },
                );
                info!(
                    "Created default room: {} ({})",
                    room_config.name, room_config.id
                );
            }

            if !state.rooms.is_empty() {
                server.persistence.save_state(&state).await.map_err(|e| {
                    error!("Failed to save initial state: {}", e);
                    e
                })?;
            }
        } else {
            info!("Restoring {} rooms from persistence", state.rooms.len());
            // Restore rooms from persistence (clear user lists as they're not currently connected)
            for (id, persisted_room) in state.rooms {
                debug!(room_id = %id, room_name = %persisted_room.name, "Restoring room");
                let room = ChatRoom {
                    id: persisted_room.id,
                    name: persisted_room.name,
                    users: HashSet::new(), // Clear users on restart
                    created_at: persisted_room.created_at,
                };
                server.rooms.insert(id, room);
            }
        }

        Ok(server)
    }

    async fn next_message_id(&self) -> u64 {
        let mut counter = self.message_counter.write().await;
        let id = *counter;
        *counter += 1;
        id
    }

    fn get_room_info(&self, room_id: &str) -> Option<RoomInfo> {
        self.rooms.get(room_id).map(|room| RoomInfo {
            room_id: room.id.clone(),
            room_name: room.name.clone(),
            user_count: room.users.len() as u32,
        })
    }

    // Clean up expired typing states (older than 5 seconds)
    async fn cleanup_expired_typing_states(&self, connection_manager: &dyn ConnectionManager) {
        let mut typing_users = self.typing_users.lock().await;
        let now = Instant::now();
        let timeout = Duration::from_secs(5);
        
        let mut expired_users = Vec::new();
        
        for (room_id, room_typing_users) in typing_users.iter_mut() {
            room_typing_users.retain(|username, state| {
                if now.duration_since(state.started_at) > timeout {
                    expired_users.push((room_id.clone(), username.clone()));
                    false
                } else {
                    true
                }
            });
        }
        
        drop(typing_users);
        
        // Send stop typing notifications for expired users
        for (room_id, username) in expired_users {
            self.broadcast_typing_notification(
                connection_manager,
                &room_id,
                &username,
                false
            ).await;
        }
    }

    // Broadcast typing notification to all users in a room
    async fn broadcast_typing_notification(
        &self,
        connection_manager: &dyn ConnectionManager,
        room_id: &str,
        username: &str,
        is_typing: bool,
    ) {
        if let Some(room) = self.rooms.get(room_id) {
            let room_users: Vec<String> = room.users.iter().cloned().collect();
            drop(room);
            
            let notification = if is_typing {
                let notification = UserStartedTypingNotification {
                    username: username.to_string(),
                    room_id: room_id.to_string(),
                };
                ras_jsonrpc_bidirectional_types::ServerNotification {
                    method: "user_started_typing".to_string(),
                    params: serde_json::to_value(&notification).unwrap(),
                    metadata: None,
                }
            } else {
                let notification = UserStoppedTypingNotification {
                    username: username.to_string(),
                    room_id: room_id.to_string(),
                };
                ras_jsonrpc_bidirectional_types::ServerNotification {
                    method: "user_stopped_typing".to_string(),
                    params: serde_json::to_value(&notification).unwrap(),
                    metadata: None,
                }
            };
            
            let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(notification);
            
            // Send to all users in the room except the typing user
            for target_username in room_users {
                if target_username != username {
                    for entry in self.user_sessions.iter() {
                        if entry.username == target_username {
                            if let Err(e) = connection_manager
                                .send_to_connection(entry.connection_id, msg.clone())
                                .await
                            {
                                warn!(target_user = %target_username, connection_id = %entry.connection_id,
                                      "Failed to send typing notification: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}

// Implement the chat service
#[async_trait::async_trait]
impl ChatServiceService for ChatServer {
    #[instrument(skip(self, connection_manager, _user), fields(client_id = %client_id, user = %_user.user_id))]
    async fn send_message(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing send_message request");

        // Validate message length
        if request.text.len() > self.config.max_message_length {
            return Err(format!(
                "Message too long. Maximum length is {} characters",
                self.config.max_message_length
            )
            .into());
        }

        // Get user session
        let session = self.user_sessions.get(&client_id).ok_or_else(|| {
            error!("User session not found for client {}", client_id);
            "User session not found"
        })?;

        let room_id = session.current_room.clone().ok_or_else(|| {
            warn!("User {} not in any room", session.username);
            "User not in any room"
        })?;

        // Drop the session ref to avoid holding the lock
        let username = session.username.clone();
        drop(session);

        // Clear typing state when sending a message
        let mut typing_users = self.typing_users.lock().await;
        let mut was_typing = false;
        if let Some(room_typing_users) = typing_users.get_mut(&room_id) {
            if room_typing_users.remove(&username).is_some() {
                was_typing = true;
            }
            if room_typing_users.is_empty() {
                typing_users.remove(&room_id);
            }
        }
        drop(typing_users);

        // Send stop typing notification if user was typing
        if was_typing {
            self.broadcast_typing_notification(
                connection_manager,
                &room_id,
                &username,
                false
            ).await;
        }

        // Get room to find all users
        let room = self.rooms.get(&room_id).ok_or_else(|| {
            error!("Room {} not found", room_id);
            "Room not found"
        })?;
        let room_users: Vec<String> = room.users.iter().cloned().collect();
        let user_count = room.users.len();
        drop(room);

        debug!(room_id = %room_id, user_count = user_count, "Broadcasting message to room");

        // Generate message details
        let message_id = self.next_message_id().await;
        let timestamp = Utc::now();
        let timestamp_str = timestamp.to_rfc3339();

        // Create notification
        let notification = MessageReceivedNotification {
            message_id,
            username: username.clone(),
            text: request.text.clone(),
            timestamp: timestamp_str.clone(),
            room_id: room_id.clone(),
        };

        // Persist message to disk
        let persisted_msg = PersistedMessage {
            id: message_id,
            room_id: room_id.clone(),
            username: username.clone(),
            text: request.text,
            timestamp,
        };
        if let Err(e) = self
            .persistence
            .append_message(&room_id, &persisted_msg)
            .await
        {
            error!(message_id = message_id, room_id = %room_id, "Failed to persist message: {}", e);
        } else {
            debug!(message_id = message_id, "Message persisted successfully");
        }

        // Send to all users in the room
        for target_username in room_users {
            // Find connection ID for this username
            for entry in self.user_sessions.iter() {
                if entry.username == target_username {
                    // Send notification directly using connection manager
                    let notification_msg = ras_jsonrpc_bidirectional_types::ServerNotification {
                        method: "message_received".to_string(),
                        params: serde_json::to_value(&notification).unwrap(),
                        metadata: None,
                    };
                    let msg =
                        ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(
                            notification_msg,
                        );
                    if let Err(e) = connection_manager
                        .send_to_connection(entry.connection_id, msg)
                        .await
                    {
                        warn!(target_user = %target_username, connection_id = %entry.connection_id,
                              "Failed to send message notification: {:?}", e);
                    }
                }
            }
        }

        info!(message_id = message_id, room_id = %room_id, sender = %username,
              "Message sent successfully");
        Ok(SendMessageResponse {
            message_id,
            timestamp: timestamp_str,
        })
    }

    #[instrument(skip(self, connection_manager, _user), fields(client_id = %client_id, user = %_user.user_id, room_name = %request.room_name))]
    async fn join_room(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: JoinRoomRequest,
    ) -> Result<JoinRoomResponse, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing join_room request");

        // Validate room name length
        if request.room_name.len() > self.config.max_room_name_length {
            return Err(format!(
                "Room name too long. Maximum length is {} characters",
                self.config.max_room_name_length
            )
            .into());
        }

        // Get or create room
        let room_id = if self.rooms.contains_key(&request.room_name) {
            request.room_name.clone()
        } else {
            // Create new room
            let room_id = if request.room_name.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                request.room_name.clone()
            };

            let new_room = ChatRoom {
                id: room_id.clone(),
                name: request.room_name.clone(),
                users: HashSet::new(),
                created_at: Utc::now(),
            };

            self.rooms.insert(room_id.clone(), new_room.clone());

            // Persist new room
            let mut state = self.persistence.load_state().await.unwrap_or_default();
            state.rooms.insert(
                room_id.clone(),
                PersistedRoom {
                    id: new_room.id.clone(),
                    name: new_room.name.clone(),
                    created_at: new_room.created_at,
                    users: new_room.users.clone(),
                },
            );
            if let Err(e) = self.persistence.save_state(&state).await {
                error!(room_id = %room_id, "Failed to persist new room: {}", e);
            } else {
                info!(room_id = %room_id, room_name = %new_room.name, "New room created and persisted");
            }

            // Notify all users about new room
            let room_info = self.get_room_info(&room_id).unwrap();
            let notification = RoomCreatedNotification { room_info };

            // Broadcast to all connected users
            for entry in self.user_sessions.iter() {
                let notification_msg = ras_jsonrpc_bidirectional_types::ServerNotification {
                    method: "room_created".to_string(),
                    params: serde_json::to_value(&notification).unwrap(),
                    metadata: None,
                };
                let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(
                    notification_msg,
                );
                if let Err(e) = connection_manager
                    .send_to_connection(entry.connection_id, msg)
                    .await
                {
                    warn!(connection_id = %entry.connection_id,
                          "Failed to send room_created notification: {:?}", e);
                }
            }

            room_id
        };

        // Get user session
        let mut session = self.user_sessions.get_mut(&client_id).ok_or_else(|| {
            error!("User session not found for client {}", client_id);
            "User session not found"
        })?;

        let username = session.username.clone();

        // Leave current room if in one
        if let Some(current_room_id) = &session.current_room {
            if let Some(mut room) = self.rooms.get_mut(current_room_id) {
                room.users.remove(&username);
                let user_count = room.users.len() as u32;
                drop(room);

                // Notify users in old room
                let notification = UserLeftNotification {
                    username: username.clone(),
                    room_id: current_room_id.clone(),
                    user_count,
                };

                for entry in self.user_sessions.iter() {
                    if entry.current_room.as_ref() == Some(current_room_id) {
                        let notification_msg =
                            ras_jsonrpc_bidirectional_types::ServerNotification {
                                method: "user_left".to_string(),
                                params: serde_json::to_value(&notification).unwrap(),
                                metadata: None,
                            };
                        let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(notification_msg);
                        if let Err(e) = connection_manager
                            .send_to_connection(entry.connection_id, msg)
                            .await
                        {
                            warn!(connection_id = %entry.connection_id,
                                  "Failed to send user_left notification: {:?}", e);
                        }
                    }
                }
            }
        }

        // Update session
        session.current_room = Some(room_id.clone());
        drop(session);

        // Add user to new room
        let mut room = self.rooms.get_mut(&room_id).ok_or("Room not found")?;

        // Check user limit
        if self.config.max_users_per_room > 0 && room.users.len() >= self.config.max_users_per_room
        {
            return Err(format!(
                "Room is full. Maximum {} users allowed per room",
                self.config.max_users_per_room
            )
            .into());
        }

        room.users.insert(username.clone());
        let user_count = room.users.len() as u32;
        let room_users: Vec<String> = room.users.iter().cloned().collect();
        drop(room);

        // Notify users in new room
        let notification = UserJoinedNotification {
            username,
            room_id: room_id.clone(),
            user_count,
        };

        for target_username in room_users {
            for entry in self.user_sessions.iter() {
                if entry.username == target_username {
                    let notification_msg = ras_jsonrpc_bidirectional_types::ServerNotification {
                        method: "user_joined".to_string(),
                        params: serde_json::to_value(&notification).unwrap(),
                        metadata: None,
                    };
                    let msg =
                        ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(
                            notification_msg,
                        );
                    if let Err(e) = connection_manager
                        .send_to_connection(entry.connection_id, msg)
                        .await
                    {
                        warn!(target_user = %target_username, connection_id = %entry.connection_id,
                              "Failed to send message notification: {:?}", e);
                    }
                }
            }
        }

        Ok(JoinRoomResponse {
            room_id,
            user_count,
        })
    }

    async fn leave_room(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: LeaveRoomRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut session = self
            .user_sessions
            .get_mut(&client_id)
            .ok_or("User session not found")?;

        // Check if user is in the requested room
        if session.current_room.as_ref() != Some(&request.room_id) {
            return Err("User not in the specified room".into());
        }

        let username = session.username.clone();
        let room_id_for_log = request.room_id.clone();
        session.current_room = None;
        drop(session);

        // Remove user from room
        if let Some(mut room) = self.rooms.get_mut(&request.room_id) {
            room.users.remove(&username);
            let user_count = room.users.len() as u32;
            let room_users: Vec<String> = room.users.iter().cloned().collect();
            drop(room);

            // Notify remaining users
            let notification = UserLeftNotification {
                username: username.clone(),
                room_id: request.room_id,
                user_count,
            };

            for target_username in room_users {
                for entry in self.user_sessions.iter() {
                    if entry.username == target_username {
                        let notification_msg =
                            ras_jsonrpc_bidirectional_types::ServerNotification {
                                method: "user_left".to_string(),
                                params: serde_json::to_value(&notification).unwrap(),
                                metadata: None,
                            };
                        let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(notification_msg);
                        if let Err(e) = connection_manager
                            .send_to_connection(entry.connection_id, msg)
                            .await
                        {
                            warn!(connection_id = %entry.connection_id,
                                  "Failed to send user_left notification: {:?}", e);
                        }
                    }
                }
            }
        }

        info!(user = %username, room_id = %room_id_for_log, "User left room successfully");
        Ok(())
    }

    #[instrument(skip(self, _connection_manager, _user), fields(client_id = %_client_id, user = %_user.user_id))]
    async fn list_rooms(
        &self,
        _client_id: ConnectionId,
        _connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        _request: ListRoomsRequest,
    ) -> Result<ListRoomsResponse, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing list_rooms request");
        let rooms: Vec<RoomInfo> = self
            .rooms
            .iter()
            .map(|entry| RoomInfo {
                room_id: entry.id.clone(),
                room_name: entry.name.clone(),
                user_count: entry.users.len() as u32,
            })
            .collect();

        debug!(room_count = rooms.len(), "Returning room list");
        Ok(ListRoomsResponse { rooms })
    }

    async fn kick_user(
        &self,
        _client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: KickUserRequest,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Find the target user's session
        let mut target_connection_id = None;
        let mut target_room_id = None;

        for entry in self.user_sessions.iter() {
            if entry.username == request.target_username {
                target_connection_id = Some(entry.connection_id);
                target_room_id = entry.current_room.clone();
                break;
            }
        }

        let target_id = target_connection_id.ok_or("Target user not found")?;

        // Remove user from their room if they're in one
        if let Some(ref room_id) = target_room_id {
            if let Some(mut room) = self.rooms.get_mut(room_id) {
                room.users.remove(&request.target_username);
            }
        }

        // Send kick notification to the target user
        let kick_notification = UserKickedNotification {
            username: request.target_username.clone(),
            reason: request.reason.clone(),
            room_id: target_room_id.as_ref().cloned().unwrap_or_default(),
        };

        let notification_msg = ras_jsonrpc_bidirectional_types::ServerNotification {
            method: "user_kicked".to_string(),
            params: serde_json::to_value(&kick_notification).unwrap(),
            metadata: None,
        };
        let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(
            notification_msg,
        );
        if let Err(e) = connection_manager.send_to_connection(target_id, msg).await {
            warn!("Failed to send kick notification to user: {:?}", e);
        }

        // Remove the user's session
        self.user_sessions.remove(&target_id);
        debug!("Removed user session for {}", request.target_username);

        // Disconnect the user
        if let Err(e) = connection_manager.remove_connection(target_id).await {
            warn!("Failed to disconnect user: {:?}", e);
        }

        Ok(true)
    }

    async fn broadcast_announcement(
        &self,
        _client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: BroadcastAnnouncementRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let notification = SystemAnnouncementNotification {
            message: request.message,
            level: request.level,
            timestamp: Utc::now().to_rfc3339(),
        };

        // Send to all connected users
        for entry in self.user_sessions.iter() {
            let notification_msg = ras_jsonrpc_bidirectional_types::ServerNotification {
                method: "system_announcement".to_string(),
                params: serde_json::to_value(&notification).unwrap(),
                metadata: None,
            };
            let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(
                notification_msg,
            );
            if let Err(e) = connection_manager
                .send_to_connection(entry.connection_id, msg)
                .await
            {
                warn!(connection_id = %entry.connection_id,
                      "Failed to send announcement: {:?}", e);
            }
        }

        let user_count = self.user_sessions.len();
        info!(user_count = user_count, "Announcement broadcast complete");
        Ok(())
    }

    async fn get_profile(
        &self,
        _client_id: ConnectionId,
        _connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: GetProfileRequest,
    ) -> Result<GetProfileResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Load current state
        let state = self.persistence.load_state().await?;

        // Get profile from persistence or create default
        let profile = if let Some(persisted) = state.user_profiles.get(&request.username) {
            UserProfile {
                username: persisted.username.clone(),
                display_name: persisted.display_name.clone(),
                avatar: CatAvatar {
                    breed: match persisted.avatar.breed.as_str() {
                        "tabby" => CatBreed::Tabby,
                        "siamese" => CatBreed::Siamese,
                        "persian" => CatBreed::Persian,
                        "maine_coon" => CatBreed::MaineCoon,
                        "british_shorthair" => CatBreed::BritishShorthair,
                        "ragdoll" => CatBreed::Ragdoll,
                        "sphynx" => CatBreed::Sphynx,
                        "scottish_fold" => CatBreed::ScottishFold,
                        "calico" => CatBreed::Calico,
                        "tuxedo" => CatBreed::Tuxedo,
                        _ => CatBreed::Tabby,
                    },
                    color: match persisted.avatar.color.as_str() {
                        "orange" => CatColor::Orange,
                        "black" => CatColor::Black,
                        "white" => CatColor::White,
                        "gray" => CatColor::Gray,
                        "brown" => CatColor::Brown,
                        "cream" => CatColor::Cream,
                        "blue" => CatColor::Blue,
                        "lilac" => CatColor::Lilac,
                        "cinnamon" => CatColor::Cinnamon,
                        "fawn" => CatColor::Fawn,
                        _ => CatColor::Orange,
                    },
                    expression: match persisted.avatar.expression.as_str() {
                        "happy" => CatExpression::Happy,
                        "sleepy" => CatExpression::Sleepy,
                        "curious" => CatExpression::Curious,
                        "playful" => CatExpression::Playful,
                        "content" => CatExpression::Content,
                        "alert" => CatExpression::Alert,
                        "grumpy" => CatExpression::Grumpy,
                        "loving" => CatExpression::Loving,
                        _ => CatExpression::Happy,
                    },
                },
                created_at: persisted.created_at.to_rfc3339(),
                last_seen: persisted.last_seen.to_rfc3339(),
            }
        } else {
            // Create default profile
            UserProfile {
                username: request.username.clone(),
                display_name: None,
                avatar: CatAvatar {
                    breed: CatBreed::Tabby,
                    color: CatColor::Orange,
                    expression: CatExpression::Happy,
                },
                created_at: Utc::now().to_rfc3339(),
                last_seen: Utc::now().to_rfc3339(),
            }
        };

        Ok(GetProfileResponse { profile })
    }

    async fn update_profile(
        &self,
        _client_id: ConnectionId,
        _connection_manager: &dyn ConnectionManager,
        user: &AuthenticatedUser,
        request: UpdateProfileRequest,
    ) -> Result<UpdateProfileResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Load current state
        let mut state = self.persistence.load_state().await?;

        // Get existing profile or create new one
        let mut persisted_profile = state
            .user_profiles
            .get(&user.user_id)
            .cloned()
            .unwrap_or_else(|| PersistedUserProfile {
                username: user.user_id.clone(),
                display_name: None,
                avatar: PersistedCatAvatar {
                    breed: "tabby".to_string(),
                    color: "orange".to_string(),
                    expression: "happy".to_string(),
                },
                created_at: Utc::now(),
                last_seen: Utc::now(),
            });

        // Update fields if provided
        if let Some(display_name) = request.display_name {
            persisted_profile.display_name = Some(display_name);
        }

        if let Some(avatar) = request.avatar {
            persisted_profile.avatar = PersistedCatAvatar {
                breed: format!("{:?}", avatar.breed).to_lowercase(),
                color: format!("{:?}", avatar.color).to_lowercase(),
                expression: format!("{:?}", avatar.expression).to_lowercase(),
            };
        }

        // Update last seen
        persisted_profile.last_seen = Utc::now();

        // Save to persistence
        state
            .user_profiles
            .insert(user.user_id.clone(), persisted_profile.clone());
        self.persistence.save_state(&state).await?;

        // Convert to response
        let profile = UserProfile {
            username: persisted_profile.username,
            display_name: persisted_profile.display_name,
            avatar: CatAvatar {
                breed: match persisted_profile.avatar.breed.as_str() {
                    "tabby" => CatBreed::Tabby,
                    "siamese" => CatBreed::Siamese,
                    "persian" => CatBreed::Persian,
                    "maine_coon" => CatBreed::MaineCoon,
                    "british_shorthair" => CatBreed::BritishShorthair,
                    "ragdoll" => CatBreed::Ragdoll,
                    "sphynx" => CatBreed::Sphynx,
                    "scottish_fold" => CatBreed::ScottishFold,
                    "calico" => CatBreed::Calico,
                    "tuxedo" => CatBreed::Tuxedo,
                    _ => CatBreed::Tabby,
                },
                color: match persisted_profile.avatar.color.as_str() {
                    "orange" => CatColor::Orange,
                    "black" => CatColor::Black,
                    "white" => CatColor::White,
                    "gray" => CatColor::Gray,
                    "brown" => CatColor::Brown,
                    "cream" => CatColor::Cream,
                    "blue" => CatColor::Blue,
                    "lilac" => CatColor::Lilac,
                    "cinnamon" => CatColor::Cinnamon,
                    "fawn" => CatColor::Fawn,
                    _ => CatColor::Orange,
                },
                expression: match persisted_profile.avatar.expression.as_str() {
                    "happy" => CatExpression::Happy,
                    "sleepy" => CatExpression::Sleepy,
                    "curious" => CatExpression::Curious,
                    "playful" => CatExpression::Playful,
                    "content" => CatExpression::Content,
                    "alert" => CatExpression::Alert,
                    "grumpy" => CatExpression::Grumpy,
                    "loving" => CatExpression::Loving,
                    _ => CatExpression::Happy,
                },
            },
            created_at: persisted_profile.created_at.to_rfc3339(),
            last_seen: persisted_profile.last_seen.to_rfc3339(),
        };

        Ok(UpdateProfileResponse { profile })
    }

    async fn start_typing(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        _request: StartTypingRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get user session
        let session = self.user_sessions.get(&client_id).ok_or_else(|| {
            error!("User session not found for client {}", client_id);
            "User session not found"
        })?;

        let username = session.username.clone();
        let room_id = session.current_room.clone().ok_or_else(|| {
            warn!("User {} not in any room", session.username);
            "User not in any room"
        })?;
        drop(session);

        // Update typing state
        let mut typing_users = self.typing_users.lock().await;
        let room_typing_users = typing_users.entry(room_id.clone()).or_insert_with(HashMap::new);
        
        let is_new_typing = !room_typing_users.contains_key(&username);
        room_typing_users.insert(username.clone(), TypingState {
            username: username.clone(),
            started_at: Instant::now(),
        });
        drop(typing_users);

        // Send notification only if this is a new typing state
        if is_new_typing {
            self.broadcast_typing_notification(
                connection_manager,
                &room_id,
                &username,
                true
            ).await;
        }

        // Clean up expired typing states
        self.cleanup_expired_typing_states(connection_manager).await;

        Ok(())
    }

    async fn stop_typing(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        _request: StopTypingRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get user session
        let session = self.user_sessions.get(&client_id).ok_or_else(|| {
            error!("User session not found for client {}", client_id);
            "User session not found"
        })?;

        let username = session.username.clone();
        let room_id = session.current_room.clone().ok_or_else(|| {
            warn!("User {} not in any room", session.username);
            "User not in any room"
        })?;
        drop(session);

        // Remove from typing state
        let mut typing_users = self.typing_users.lock().await;
        let mut should_notify = false;
        
        if let Some(room_typing_users) = typing_users.get_mut(&room_id) {
            if room_typing_users.remove(&username).is_some() {
                should_notify = true;
            }
            
            // Clean up empty room entries
            if room_typing_users.is_empty() {
                typing_users.remove(&room_id);
            }
        }
        drop(typing_users);

        // Send notification if user was typing
        if should_notify {
            self.broadcast_typing_notification(
                connection_manager,
                &room_id,
                &username,
                false
            ).await;
        }

        Ok(())
    }

    // Notification stub methods (required by the trait but not used by server)
    async fn notify_message_received(
        &self,
        _connection_id: ConnectionId,
        _params: MessageReceivedNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_user_joined(
        &self,
        _connection_id: ConnectionId,
        _params: UserJoinedNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_user_left(
        &self,
        _connection_id: ConnectionId,
        _params: UserLeftNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_system_announcement(
        &self,
        _connection_id: ConnectionId,
        _params: SystemAnnouncementNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_user_kicked(
        &self,
        _connection_id: ConnectionId,
        _params: UserKickedNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_room_created(
        &self,
        _connection_id: ConnectionId,
        _params: RoomCreatedNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_room_deleted(
        &self,
        _connection_id: ConnectionId,
        _params: RoomDeletedNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_user_started_typing(
        &self,
        _connection_id: ConnectionId,
        _params: UserStartedTypingNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    async fn notify_user_stopped_typing(
        &self,
        _connection_id: ConnectionId,
        _params: UserStoppedTypingNotification,
    ) -> ras_jsonrpc_bidirectional_types::Result<()> {
        Ok(())
    }

    // Lifecycle hooks
    async fn on_client_connected(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Client {} connected", client_id);

        // Send welcome message
        let notification = SystemAnnouncementNotification {
            message: "Welcome to the chat server! Please authenticate to continue.".to_string(),
            level: AnnouncementLevel::Info,
            timestamp: Utc::now().to_rfc3339(),
        };

        let notification_msg = ras_jsonrpc_bidirectional_types::ServerNotification {
            method: "system_announcement".to_string(),
            params: serde_json::to_value(&notification).unwrap(),
            metadata: None,
        };
        let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(
            notification_msg,
        );
        if let Err(e) = connection_manager.send_to_connection(client_id, msg).await {
            warn!(
                "Failed to send welcome message to client {}: {:?}",
                client_id, e
            );
        }

        Ok(())
    }

    async fn on_client_disconnected(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Client {} disconnected", client_id);

        // Remove user session and notify room members
        if let Some((_, session)) = self.user_sessions.remove(&client_id) {
            let username = session.username.clone();
            
            if let Some(room_id) = session.current_room {
                // Clear typing state if user was typing
                let mut typing_users = self.typing_users.lock().await;
                let mut was_typing = false;
                if let Some(room_typing_users) = typing_users.get_mut(&room_id) {
                    if room_typing_users.remove(&username).is_some() {
                        was_typing = true;
                    }
                    if room_typing_users.is_empty() {
                        typing_users.remove(&room_id);
                    }
                }
                drop(typing_users);

                // Send stop typing notification if user was typing
                if was_typing {
                    self.broadcast_typing_notification(
                        connection_manager,
                        &room_id,
                        &username,
                        false
                    ).await;
                }
                
                // Remove from room
                if let Some(mut room) = self.rooms.get_mut(&room_id) {
                    room.users.remove(&session.username);
                    let user_count = room.users.len() as u32;
                    let room_users: Vec<String> = room.users.iter().cloned().collect();
                    drop(room);

                    // Notify remaining users
                    let notification = UserLeftNotification {
                        username: session.username,
                        room_id,
                        user_count,
                    };

                    for target_username in room_users {
                        for entry in self.user_sessions.iter() {
                            if entry.username == target_username {
                                let notification_msg =
                                    ras_jsonrpc_bidirectional_types::ServerNotification {
                                        method: "user_left".to_string(),
                                        params: serde_json::to_value(&notification).unwrap(),
                                        metadata: None,
                                    };
                                let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(notification_msg);
                                if let Err(e) = connection_manager
                                    .send_to_connection(entry.connection_id, msg)
                                    .await
                                {
                                    warn!(connection_id = %entry.connection_id,
                                          "Failed to send user_left notification on disconnect: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn on_client_authenticated(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        user: &AuthenticatedUser,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Client {} authenticated as user {}",
            client_id, user.user_id
        );

        // Create user session
        let session = UserSession {
            username: user.user_id.clone(),
            connection_id: client_id,
            current_room: None,
            joined_at: Utc::now(),
        };

        self.user_sessions.insert(client_id, session);

        // Send personalized welcome
        let notification = SystemAnnouncementNotification {
            message: format!(
                "Welcome {}, you have been successfully authenticated!",
                user.user_id
            ),
            level: AnnouncementLevel::Info,
            timestamp: Utc::now().to_rfc3339(),
        };

        let notification_msg = ras_jsonrpc_bidirectional_types::ServerNotification {
            method: "system_announcement".to_string(),
            params: serde_json::to_value(&notification).unwrap(),
            metadata: None,
        };
        let msg = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(
            notification_msg,
        );
        if let Err(e) = connection_manager.send_to_connection(client_id, msg).await {
            warn!(
                "Failed to send welcome message to client {}: {:?}",
                client_id, e
            );
        }

        Ok(())
    }
}

// Permission provider for the chat application
#[derive(Clone)]
struct ChatPermissions {
    admin_users: Vec<config::AdminUser>,
}

// REST API handlers
#[derive(Clone)]
struct AuthHandlers {
    session_service: Arc<SessionService>,
    identity_provider: Arc<LocalUserProvider>,
}

impl ChatPermissions {
    fn new(admin_users: Vec<config::AdminUser>) -> Self {
        Self { admin_users }
    }
}

#[async_trait::async_trait]
impl UserPermissions for ChatPermissions {
    async fn get_permissions(
        &self,
        identity: &VerifiedIdentity,
    ) -> ras_identity_core::IdentityResult<Vec<String>> {
        // Check if user is in admin configuration
        for admin_user in &self.admin_users {
            if admin_user.username == identity.subject {
                return Ok(admin_user.permissions.clone());
            }
        }

        // Default permissions for regular users
        Ok(vec!["user".to_string()])
    }
}

impl AuthHandlers {
    async fn handle_login(
        &self,
        request: LoginRequest,
    ) -> Result<LoginResponse, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing login request");

        // Create auth payload
        let provider_id = request.provider.as_deref().unwrap_or("local");
        let auth_payload = json!({
            "username": request.username,
            "password": request.password,
            "provider": provider_id,
        });

        // Begin session
        let token = self
            .session_service
            .begin_session(provider_id, auth_payload)
            .await
            .map_err(|e| {
                warn!(provider = %provider_id, "Login failed: {}", e);
                format!("Authentication failed: {}", e)
            })?;

        // Parse token to get user info (for response)
        let claims = self
            .session_service
            .verify_session(&token)
            .await
            .map_err(|e| {
                warn!("Token verification failed: {}", e);
                format!("Token verification failed: {}", e)
            })?;

        info!(user_id = %claims.sub, "User logged in successfully");
        Ok(LoginResponse {
            token,
            expires_at: claims.exp,
            user_id: claims.sub,
        })
    }

    async fn handle_register(
        &self,
        request: RegisterRequest,
    ) -> Result<RegisterResponse, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing registration request");

        // Add user
        self.identity_provider
            .add_user(
                request.username.clone(),
                request.password,
                request.email.clone(),
                request.display_name.clone(),
            )
            .await
            .map_err(|e| {
                warn!(username = %request.username, "Registration failed: {}", e);
                format!("Registration failed: {}", e)
            })?;

        info!(username = %request.username, email = ?request.email, "User registered successfully");

        Ok(RegisterResponse {
            message: "User registered successfully".to_string(),
            username: request.username,
            display_name: request.display_name,
        })
    }

    async fn handle_health(
        &self,
    ) -> Result<HealthResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HealthResponse {
            status: "OK".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables first (before config loading)
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("No .env file found or error loading: {}", e);
    }

    // Load configuration
    let config = Config::load().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;

    // Initialize tracing based on configuration
    use tracing_subscriber::{EnvFilter, fmt};

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new(config.log_filter()))
        .with_target(config.logging.target)
        .with_thread_ids(config.logging.thread_ids)
        .with_line_number(config.logging.line_numbers)
        .with_level(true)
        .with_ansi(true);

    // Apply format settings
    match config.logging.format.as_str() {
        "json" => {
            subscriber.with_ansi(false).init();
        }
        "compact" => {
            subscriber.compact().init();
        }
        _ => {
            // "pretty" or default
            subscriber.pretty().init();
        }
    }

    info!("Starting bidirectional chat server");
    info!("Configuration loaded from environment and config file");

    // Create identity provider - use Arc to share between session service and registration
    info!("Setting up identity provider");
    let identity_provider = Arc::new(LocalUserProvider::new());

    // Add admin users from configuration
    if config.admin.auto_create {
        for admin_user in &config.admin.users {
            match identity_provider
                .add_user(
                    admin_user.username.clone(),
                    admin_user.password.clone(),
                    admin_user.email.clone(),
                    admin_user.display_name.clone(),
                )
                .await
            {
                Ok(_) => info!("Created admin user: {}", admin_user.username),
                Err(e) => {
                    // User might already exist, which is fine
                    debug!(
                        "Admin user {} might already exist: {}",
                        admin_user.username, e
                    );
                }
            }
        }
    }

    // Add some default test users if in development mode
    if cfg!(debug_assertions) {
        let test_users = vec![
            (
                "alice",
                "alice123",
                Some("alice@example.com"),
                Some("Alice"),
            ),
            ("bob", "bob123", Some("bob@example.com"), Some("Bob")),
        ];

        for (username, password, email, display_name) in test_users {
            match identity_provider
                .add_user(
                    username.to_string(),
                    password.to_string(),
                    email.map(|s| s.to_string()),
                    display_name.map(|s| s.to_string()),
                )
                .await
            {
                Ok(_) => debug!("Created test user: {}", username),
                Err(e) => debug!("Test user {} might already exist: {}", username, e),
            }
        }
    }

    // Create session service from configuration
    let session_config = SessionConfig {
        jwt_secret: config.auth.jwt_secret.clone(),
        jwt_ttl: chrono::Duration::seconds(config.auth.jwt_ttl_seconds),
        refresh_enabled: config.auth.refresh_enabled,
        algorithm: match config.auth.jwt_algorithm.as_str() {
            "HS256" => jsonwebtoken::Algorithm::HS256,
            "HS384" => jsonwebtoken::Algorithm::HS384,
            "HS512" => jsonwebtoken::Algorithm::HS512,
            _ => jsonwebtoken::Algorithm::HS256, // Default
        },
    };
    info!(
        "Creating session service with JWT TTL: {} seconds",
        config.auth.jwt_ttl_seconds
    );
    let session_service = Arc::new(
        SessionService::new(session_config)
            .with_permissions(Arc::new(ChatPermissions::new(config.admin.users.clone()))),
    );

    // Register the identity provider with the session service
    // We need to dereference the Arc and clone the inner provider since register_provider takes Box
    session_service
        .register_provider(Box::new((*identity_provider).clone()))
        .await;

    // Create JWT auth provider
    let auth_provider = Arc::new(JwtAuthProvider::new(session_service.clone()));

    // Create connection manager
    let connection_manager = Arc::new(DefaultConnectionManager::new());

    // Create chat server with configuration
    let chat_server = Arc::new(ChatServer::new(config.chat.clone()).await.map_err(|e| {
        error!("Failed to create chat server: {}", e);
        e
    })?);

    // Create handler with the service and connection manager
    let handler = Arc::new(bidirectional_chat_api::ChatServiceHandler::new(
        chat_server.clone(),
        connection_manager.clone(),
    ));

    // Build WebSocket service
    let ws_service = WebSocketServiceBuilder::builder()
        .handler(handler)
        .auth_provider(auth_provider.clone())
        .require_auth(true)
        .build()
        .build_with_manager(connection_manager);

    // Create auth handlers with the shared identity provider
    let auth_handlers = AuthHandlers {
        session_service: session_service.clone(),
        identity_provider: identity_provider.clone(),
    };

    // Build REST service using the macro-generated builder
    let auth_handlers_clone1 = auth_handlers.clone();
    let auth_handlers_clone2 = auth_handlers.clone();
    let auth_handlers_clone3 = auth_handlers.clone();

    let auth_router = ChatAuthServiceBuilder::new()
        .auth_provider(auth_provider.as_ref().clone())
        .post_auth_login_handler(move |req| {
            let handlers = auth_handlers_clone1.clone();
            async move { handlers.handle_login(req).await }
        })
        .post_auth_register_handler(move |req| {
            let handlers = auth_handlers_clone2.clone();
            async move { handlers.handle_register(req).await }
        })
        .get_health_handler(move || {
            let handlers = auth_handlers_clone3.clone();
            async move { handlers.handle_health().await }
        })
        .build();

    // Create WebSocket endpoint
    type ChatServiceType = BuiltWebSocketService<
        bidirectional_chat_api::ChatServiceHandler<ChatServer, DefaultConnectionManager>,
        JwtAuthProvider,
        DefaultConnectionManager,
    >;
    let ws_router = Router::new()
        .route("/ws", get(websocket_handler::<ChatServiceType>))
        .with_state(ws_service);

    // Configure CORS based on configuration
    let cors_layer = if config.server.cors.allow_any_origin {
        CorsLayer::permissive()
    } else {
        let mut cors = CorsLayer::new();
        for origin in &config.server.cors.allowed_origins {
            cors = cors.allow_origin(origin.parse::<axum::http::HeaderValue>().unwrap());
        }
        cors
    };

    // Combine all routers
    let app = Router::new()
        .merge(auth_router)
        .merge(ws_router)
        .layer(cors_layer);

    // Start server
    let addr = config.socket_addr();

    info!("Chat server listening on http://{}", addr);
    info!("WebSocket endpoint: ws://{}/ws", addr);
    info!("Health check endpoint: http://{}/health", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        error!("Failed to bind to address {}: {}", addr, e);
        e
    })?;

    info!("Server started successfully, ready to accept connections");

    axum::serve(listener, app).await.map_err(|e| {
        error!("Server error: {}", e);
        e
    })?;

    Ok(())
}
