use anyhow::Result;
use bidirectional_chat_api::{
    ChatServiceClient, ChatServiceClientBuilder, JoinRoomRequest, LeaveRoomRequest,
    ListRoomsRequest, MessageReceivedNotification, RoomInfo, SendMessageRequest,
    StartTypingRequest, StopTypingRequest, SystemAnnouncementNotification, 
    UserJoinedNotification, UserLeftNotification, UserStartedTypingNotification,
    UserStoppedTypingNotification,
};
use chrono::{DateTime, Local};
use tokio::sync::mpsc;
use crate::avatar::AvatarManager;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub username: String,
    pub text: String,
    pub timestamp: DateTime<Local>,
    pub room_id: String,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    MessageReceived(Message),
    UserJoined { username: String, room_id: String },
    UserLeft { username: String, room_id: String },
    UserStartedTyping { username: String, room_id: String },
    UserStoppedTyping { username: String, room_id: String },
    SystemAnnouncement { message: String },
    RoomListUpdated(Vec<RoomInfo>),
    Error(String),
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppScreen {
    Login,
    Register,
    RoomList,
    Chat { room_id: String, room_name: String },
}

pub struct AppState {
    pub screen: AppScreen,
    pub messages: Vec<Message>,
    pub rooms: Vec<RoomInfo>,
    pub current_room: Option<(String, String)>, // (room_id, room_name)
    pub username: Option<String>,
    pub error_message: Option<String>,
    pub input_buffer: String,
    pub auth_username_input: String,
    pub auth_password_input: String,
    pub auth_field_focus: AuthField,
    pub connected: bool,
    pub avatar_manager: AvatarManager,
    pub room_users: std::collections::HashMap<String, Vec<String>>, // room_id -> list of users
    pub typing_users: std::collections::HashMap<String, std::collections::HashSet<String>>, // room_id -> set of typing users
    pub last_typing_time: Option<std::time::Instant>,
    pub is_typing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthField {
    Username,
    Password,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: AppScreen::Login,
            messages: Vec::new(),
            rooms: Vec::new(),
            current_room: None,
            username: None,
            error_message: None,
            input_buffer: String::new(),
            auth_username_input: String::new(),
            auth_password_input: String::new(),
            auth_field_focus: AuthField::Username,
            connected: false,
            avatar_manager: AvatarManager::new(),
            room_users: std::collections::HashMap::new(),
            typing_users: std::collections::HashMap::new(),
            last_typing_time: None,
            is_typing: false,
        }
    }
}

pub struct ChatClient {
    client: Option<ChatServiceClient>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl ChatClient {
    pub fn new(event_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self {
            client: None,
            event_tx,
        }
    }

    pub async fn connect(&mut self, server_url: &str, jwt_token: String) -> Result<()> {
        let ws_url = format!("{}/ws", server_url.replace("http://", "ws://").replace("https://", "wss://"));
        
        let mut client = ChatServiceClientBuilder::new(ws_url)
            .with_jwt_token(jwt_token)
            .build()
            .await?;

        self.setup_event_handlers(&mut client);
        
        client.connect().await?;
        
        self.client = Some(client);
        self.event_tx.send(AppEvent::Connected)?;
        
        Ok(())
    }

    fn setup_event_handlers(&self, client: &mut ChatServiceClient) {
        let tx = self.event_tx.clone();
        client.on_message_received(move |notification: MessageReceivedNotification| {
            let message = Message {
                id: notification.message_id,
                username: notification.username,
                text: notification.text,
                timestamp: DateTime::parse_from_rfc3339(&notification.timestamp)
                    .unwrap_or_else(|_| Local::now().into())
                    .with_timezone(&Local),
                room_id: notification.room_id,
            };
            let _ = tx.send(AppEvent::MessageReceived(message));
        });

        let tx = self.event_tx.clone();
        client.on_user_joined(move |notification: UserJoinedNotification| {
            let _ = tx.send(AppEvent::UserJoined {
                username: notification.username,
                room_id: notification.room_id,
            });
        });

        let tx = self.event_tx.clone();
        client.on_user_left(move |notification: UserLeftNotification| {
            let _ = tx.send(AppEvent::UserLeft {
                username: notification.username,
                room_id: notification.room_id,
            });
        });

        let tx = self.event_tx.clone();
        client.on_system_announcement(move |notification: SystemAnnouncementNotification| {
            let _ = tx.send(AppEvent::SystemAnnouncement {
                message: notification.message,
            });
        });

        let tx = self.event_tx.clone();
        client.on_user_started_typing(move |notification: UserStartedTypingNotification| {
            let _ = tx.send(AppEvent::UserStartedTyping {
                username: notification.username,
                room_id: notification.room_id,
            });
        });

        let tx = self.event_tx.clone();
        client.on_user_stopped_typing(move |notification: UserStoppedTypingNotification| {
            let _ = tx.send(AppEvent::UserStoppedTyping {
                username: notification.username,
                room_id: notification.room_id,
            });
        });
    }

    pub async fn list_rooms(&self) -> Result<Vec<RoomInfo>> {
        match &self.client {
            Some(client) => {
                let response = client.list_rooms(ListRoomsRequest {}).await?;
                Ok(response.rooms)
            }
            None => anyhow::bail!("Not connected"),
        }
    }

    pub async fn join_room(&self, room_name: String) -> Result<(String, String)> {
        match &self.client {
            Some(client) => {
                let response = client.join_room(JoinRoomRequest { room_name: room_name.clone() }).await?;
                Ok((response.room_id, room_name))
            }
            None => anyhow::bail!("Not connected"),
        }
    }

    pub async fn leave_room(&self, room_id: String) -> Result<()> {
        match &self.client {
            Some(client) => {
                client.leave_room(LeaveRoomRequest { room_id }).await?;
                Ok(())
            }
            None => anyhow::bail!("Not connected"),
        }
    }

    pub async fn send_message(&self, text: String) -> Result<()> {
        match &self.client {
            Some(client) => {
                client.send_message(SendMessageRequest { text }).await?;
                Ok(())
            }
            None => anyhow::bail!("Not connected"),
        }
    }

    pub async fn start_typing(&self) -> Result<()> {
        match &self.client {
            Some(client) => {
                client.start_typing(StartTypingRequest {}).await?;
                Ok(())
            }
            None => anyhow::bail!("Not connected"),
        }
    }

    pub async fn stop_typing(&self) -> Result<()> {
        match &self.client {
            Some(client) => {
                client.stop_typing(StopTypingRequest {}).await?;
                Ok(())
            }
            None => anyhow::bail!("Not connected"),
        }
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            client.disconnect().await?;
            self.event_tx.send(AppEvent::Disconnected)?;
        }
        Ok(())
    }
}