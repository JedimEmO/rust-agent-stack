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
use std::{collections::HashSet, net::SocketAddr, path::Path, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use uuid::Uuid;

mod persistence;
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

// Chat server state
#[derive(Clone)]
struct ChatServer {
    rooms: Arc<DashMap<String, ChatRoom>>,
    user_sessions: Arc<DashMap<ConnectionId, UserSession>>,
    message_counter: Arc<RwLock<u64>>,
    persistence: Arc<PersistenceManager>,
}

impl ChatServer {
    async fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let persistence = Arc::new(PersistenceManager::new(data_dir));
        persistence.init().await?;

        // Load persisted state
        let mut state = persistence.load_state().await?;

        let server = Self {
            rooms: Arc::new(DashMap::new()),
            user_sessions: Arc::new(DashMap::new()),
            message_counter: Arc::new(RwLock::new(state.next_message_id)),
            persistence,
        };

        // Restore rooms
        if state.rooms.is_empty() {
            // Create default room if none exist
            let default_room = ChatRoom {
                id: "general".to_string(),
                name: "General".to_string(),
                users: HashSet::new(),
                created_at: Utc::now(),
            };
            server
                .rooms
                .insert("general".to_string(), default_room.clone());

            // Persist the default room
            state.rooms.insert(
                "general".to_string(),
                PersistedRoom {
                    id: default_room.id,
                    name: default_room.name,
                    created_at: default_room.created_at,
                    users: default_room.users.clone(),
                },
            );
            server.persistence.save_state(&state).await?;
        } else {
            // Restore rooms from persistence (clear user lists as they're not currently connected)
            for (id, persisted_room) in state.rooms {
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
}

// Implement the chat service
#[async_trait::async_trait]
impl ChatServiceService for ChatServer {
    async fn send_message(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Get user session
        let session = self
            .user_sessions
            .get(&client_id)
            .ok_or("User session not found")?;

        let room_id = session.current_room.clone().ok_or("User not in any room")?;

        // Drop the session ref to avoid holding the lock
        let username = session.username.clone();
        drop(session);

        // Get room to find all users
        let room = self.rooms.get(&room_id).ok_or("Room not found")?;
        let room_users: Vec<String> = room.users.iter().cloned().collect();
        drop(room);

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
            warn!("Failed to persist message: {}", e);
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
                    let _ = connection_manager
                        .send_to_connection(entry.connection_id, msg)
                        .await;
                }
            }
        }

        Ok(SendMessageResponse {
            message_id,
            timestamp: timestamp_str,
        })
    }

    async fn join_room(
        &self,
        client_id: ConnectionId,
        connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        request: JoinRoomRequest,
    ) -> Result<JoinRoomResponse, Box<dyn std::error::Error + Send + Sync>> {
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
                warn!("Failed to persist new room: {}", e);
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
                let _ = connection_manager
                    .send_to_connection(entry.connection_id, msg)
                    .await;
            }

            room_id
        };

        // Get user session
        let mut session = self
            .user_sessions
            .get_mut(&client_id)
            .ok_or("User session not found")?;

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
                        let _ = connection_manager
                            .send_to_connection(entry.connection_id, msg)
                            .await;
                    }
                }
            }
        }

        // Update session
        session.current_room = Some(room_id.clone());
        drop(session);

        // Add user to new room
        let mut room = self.rooms.get_mut(&room_id).ok_or("Room not found")?;
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
                    let _ = connection_manager
                        .send_to_connection(entry.connection_id, msg)
                        .await;
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
                username,
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
                        let _ = connection_manager
                            .send_to_connection(entry.connection_id, msg)
                            .await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn list_rooms(
        &self,
        _client_id: ConnectionId,
        _connection_manager: &dyn ConnectionManager,
        _user: &AuthenticatedUser,
        _request: ListRoomsRequest,
    ) -> Result<ListRoomsResponse, Box<dyn std::error::Error + Send + Sync>> {
        let rooms: Vec<RoomInfo> = self
            .rooms
            .iter()
            .map(|entry| RoomInfo {
                room_id: entry.id.clone(),
                room_name: entry.name.clone(),
                user_count: entry.users.len() as u32,
            })
            .collect();

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
        let _ = connection_manager.send_to_connection(target_id, msg).await;

        // Remove the user's session
        self.user_sessions.remove(&target_id);

        // Disconnect the user
        let _ = connection_manager.remove_connection(target_id).await;

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
            let _ = connection_manager
                .send_to_connection(entry.connection_id, msg)
                .await;
        }

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
        let _ = connection_manager.send_to_connection(client_id, msg).await;

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
            if let Some(room_id) = session.current_room {
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
                                let _ = connection_manager
                                    .send_to_connection(entry.connection_id, msg)
                                    .await;
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
        let _ = connection_manager.send_to_connection(client_id, msg).await;

        Ok(())
    }
}

// Permission provider for the chat application
#[derive(Clone)]
struct ChatPermissions;

#[async_trait::async_trait]
impl UserPermissions for ChatPermissions {
    async fn get_permissions(
        &self,
        identity: &VerifiedIdentity,
    ) -> ras_identity_core::IdentityResult<Vec<String>> {
        // In a real app, this would query a database
        let permissions = match identity.subject.as_str() {
            "admin" => vec![
                "admin".to_string(),
                "moderator".to_string(),
                "user".to_string(),
            ],
            "moderator" => vec!["moderator".to_string(), "user".to_string()],
            _ => vec!["user".to_string()],
        };
        Ok(permissions)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_ansi(true)
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Create identity provider with some test users
    let identity_provider = Arc::new(LocalUserProvider::new());
    identity_provider
        .add_user(
            "admin".to_string(),
            "admin123".to_string(),
            Some("admin@example.com".to_string()),
            Some("Administrator".to_string()),
        )
        .await?;
    identity_provider
        .add_user(
            "moderator".to_string(),
            "mod123".to_string(),
            Some("mod@example.com".to_string()),
            Some("Moderator".to_string()),
        )
        .await?;
    identity_provider
        .add_user(
            "alice".to_string(),
            "alice123".to_string(),
            Some("alice@example.com".to_string()),
            Some("Alice".to_string()),
        )
        .await?;
    identity_provider
        .add_user(
            "bob".to_string(),
            "bob123".to_string(),
            Some("bob@example.com".to_string()),
            Some("Bob".to_string()),
        )
        .await?;

    // Create session service
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-key".to_string());
    let session_config = SessionConfig {
        jwt_secret,
        jwt_ttl: chrono::Duration::hours(24),
        refresh_enabled: true,
        algorithm: jsonwebtoken::Algorithm::HS256,
    };
    let session_service =
        Arc::new(SessionService::new(session_config).with_permissions(Arc::new(ChatPermissions)));

    // Create a new LocalUserProvider for the session service
    let session_identity_provider = LocalUserProvider::new();
    // Copy users from the main identity provider
    session_identity_provider
        .add_user(
            "admin".to_string(),
            "admin123".to_string(),
            Some("admin@example.com".to_string()),
            Some("Administrator".to_string()),
        )
        .await?;
    session_identity_provider
        .add_user(
            "moderator".to_string(),
            "mod123".to_string(),
            Some("mod@example.com".to_string()),
            Some("Moderator".to_string()),
        )
        .await?;
    session_identity_provider
        .add_user(
            "alice".to_string(),
            "alice123".to_string(),
            Some("alice@example.com".to_string()),
            Some("Alice".to_string()),
        )
        .await?;
    session_identity_provider
        .add_user(
            "bob".to_string(),
            "bob123".to_string(),
            Some("bob@example.com".to_string()),
            Some("Bob".to_string()),
        )
        .await?;

    session_service
        .register_provider(Box::new(session_identity_provider))
        .await;

    // Create JWT auth provider
    let auth_provider = JwtAuthProvider::new(session_service.clone());

    // Create connection manager
    let connection_manager = Arc::new(DefaultConnectionManager::new());

    // Create chat server with data directory for persistence
    let data_dir = std::env::var("CHAT_DATA_DIR").unwrap_or_else(|_| "./chat_data".to_string());
    let chat_server = Arc::new(ChatServer::new(&data_dir).await?);

    // Create handler with the service and connection manager
    let handler = Arc::new(bidirectional_chat_api::ChatServiceHandler::new(
        chat_server.clone(),
        connection_manager.clone(),
    ));

    // Build WebSocket service
    let ws_service = WebSocketServiceBuilder::builder()
        .handler(handler)
        .auth_provider(Arc::new(auth_provider.clone()))
        .require_auth(true)
        .build()
        .build_with_manager(connection_manager);

    // Create auth endpoints for login and registration
    let auth_router = Router::new()
        .route("/auth/login", axum::routing::post(login_handler))
        .route("/auth/register", axum::routing::post(register_handler))
        .with_state((session_service, identity_provider, chat_server.clone()));

    // Create WebSocket endpoint
    type ChatServiceType = BuiltWebSocketService<
        bidirectional_chat_api::ChatServiceHandler<ChatServer, DefaultConnectionManager>,
        JwtAuthProvider,
        DefaultConnectionManager,
    >;
    let ws_router = Router::new()
        .route("/ws", get(websocket_handler::<ChatServiceType>))
        .with_state(ws_service);

    // Create health check endpoint
    let health_router = Router::new().route("/health", get(|| async { "OK" }));

    // Combine all routers
    let app = Router::new()
        .merge(auth_router)
        .merge(ws_router)
        .merge(health_router)
        .layer(CorsLayer::permissive());

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Chat server listening on http://{}", addr);
    info!("WebSocket endpoint: ws://{}/ws", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Login handler for authentication
async fn login_handler(
    axum::extract::State((session_service, _identity_provider, _chat_server)): axum::extract::State<
        (Arc<SessionService>, Arc<LocalUserProvider>, Arc<ChatServer>),
    >,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    // Extract provider_id from payload (default to "local")
    let provider_id = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("local");

    // Begin session
    let token = session_service
        .begin_session(provider_id, payload.clone())
        .await
        .map_err(|e| {
            warn!("Login failed: {}", e);
            axum::http::StatusCode::UNAUTHORIZED
        })?;

    // Parse token to get user info (for response)
    let claims = session_service.verify_session(&token).await.map_err(|e| {
        warn!("Token verification failed: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(axum::Json(json!({
        "token": token,
        "expires_at": claims.exp,
        "user_id": claims.sub,
    })))
}

// Register handler for user registration
async fn register_handler(
    axum::extract::State((_session_service, identity_provider, _chat_server)): axum::extract::State<
        (Arc<SessionService>, Arc<LocalUserProvider>, Arc<ChatServer>),
    >,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    // Extract username and password
    let username = payload
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or(axum::http::StatusCode::BAD_REQUEST)?;

    let password = payload
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or(axum::http::StatusCode::BAD_REQUEST)?;

    let email = payload
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let display_name = payload
        .get("display_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Add user
    identity_provider
        .add_user(
            username.to_string(),
            password.to_string(),
            email,
            display_name.clone(),
        )
        .await
        .map_err(|e| {
            warn!("Registration failed: {}", e);
            axum::http::StatusCode::CONFLICT // User already exists
        })?;

    Ok(axum::Json(json!({
        "message": "User registered successfully",
        "username": username,
        "display_name": display_name,
    })))
}
