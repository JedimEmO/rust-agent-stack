//! Shared types and generated service for the bidirectional chat example

pub mod auth;

use ras_jsonrpc_bidirectional_macro::jsonrpc_bidirectional_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// User profile types

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserProfile {
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: CatAvatar,
    pub created_at: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CatAvatar {
    pub breed: CatBreed,
    pub color: CatColor,
    pub expression: CatExpression,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatBreed {
    Tabby,
    Siamese,
    Persian,
    MaineCoon,
    BritishShorthair,
    Ragdoll,
    Sphynx,
    ScottishFold,
    Calico,
    Tuxedo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatColor {
    Orange,
    Black,
    White,
    Gray,
    Brown,
    Cream,
    Blue,
    Lilac,
    Cinnamon,
    Fawn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatExpression {
    Happy,
    Sleepy,
    Curious,
    Playful,
    Content,
    Alert,
    Grumpy,
    Loving,
}

// Client -> Server request types

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendMessageRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartTypingRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StopTypingRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendMessageResponse {
    pub message_id: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JoinRoomRequest {
    pub room_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JoinRoomResponse {
    pub room_id: String,
    pub user_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LeaveRoomRequest {
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListRoomsRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListRoomsResponse {
    pub rooms: Vec<RoomInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RoomInfo {
    pub room_id: String,
    pub room_name: String,
    pub user_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KickUserRequest {
    pub target_username: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BroadcastAnnouncementRequest {
    pub message: String,
    pub level: AnnouncementLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AnnouncementLevel {
    Info,
    Warning,
    Error,
}

// Profile management types

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetProfileRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetProfileResponse {
    pub profile: UserProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub avatar: Option<CatAvatar>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateProfileResponse {
    pub profile: UserProfile,
}

// Server -> Client notification types

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageReceivedNotification {
    pub message_id: u64,
    pub username: String,
    pub text: String,
    pub timestamp: String,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserJoinedNotification {
    pub username: String,
    pub room_id: String,
    pub user_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserLeftNotification {
    pub username: String,
    pub room_id: String,
    pub user_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SystemAnnouncementNotification {
    pub message: String,
    pub level: AnnouncementLevel,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserKickedNotification {
    pub username: String,
    pub reason: String,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RoomCreatedNotification {
    pub room_info: RoomInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RoomDeletedNotification {
    pub room_id: String,
    pub room_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserStartedTypingNotification {
    pub username: String,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserStoppedTypingNotification {
    pub username: String,
    pub room_id: String,
}

// Generate the bidirectional chat service
jsonrpc_bidirectional_service!({
    service_name: ChatService,

    // Client -> Server methods (with authentication/permissions)
    client_to_server: [
        WITH_PERMISSIONS(["user"]) send_message(SendMessageRequest) -> SendMessageResponse,
        WITH_PERMISSIONS(["user"]) join_room(JoinRoomRequest) -> JoinRoomResponse,
        WITH_PERMISSIONS(["user"]) leave_room(LeaveRoomRequest) -> (),
        WITH_PERMISSIONS(["user"]) list_rooms(ListRoomsRequest) -> ListRoomsResponse,
        WITH_PERMISSIONS(["moderator"]) kick_user(KickUserRequest) -> bool,
        WITH_PERMISSIONS(["admin"]) broadcast_announcement(BroadcastAnnouncementRequest) -> (),
        WITH_PERMISSIONS(["user"]) get_profile(GetProfileRequest) -> GetProfileResponse,
        WITH_PERMISSIONS(["user"]) update_profile(UpdateProfileRequest) -> UpdateProfileResponse,
        WITH_PERMISSIONS(["user"]) start_typing(StartTypingRequest) -> (),
        WITH_PERMISSIONS(["user"]) stop_typing(StopTypingRequest) -> (),
    ],

    // Server -> Client notifications (no response expected)
    server_to_client: [
        message_received(MessageReceivedNotification),
        user_joined(UserJoinedNotification),
        user_left(UserLeftNotification),
        system_announcement(SystemAnnouncementNotification),
        user_kicked(UserKickedNotification),
        room_created(RoomCreatedNotification),
        room_deleted(RoomDeletedNotification),
        user_started_typing(UserStartedTypingNotification),
        user_stopped_typing(UserStoppedTypingNotification),
    ],

    // Server -> Client calls (no bidirectional calls for this example)
    server_to_client_calls: [
    ]
});
