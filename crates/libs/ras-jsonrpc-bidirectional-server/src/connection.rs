//! Connection context and management

use ras_auth_core::AuthenticatedUser;
use ras_jsonrpc_bidirectional_types::{BidirectionalMessage, ConnectionId, ConnectionInfo};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// A simple channel-based message sender
#[derive(Debug, Clone)]
pub struct ChannelMessageSender {
    connection_id: ConnectionId,
    sender: mpsc::UnboundedSender<BidirectionalMessage>,
}

impl ChannelMessageSender {
    /// Create a new channel message sender
    pub fn new(
        connection_id: ConnectionId,
        sender: mpsc::UnboundedSender<BidirectionalMessage>,
    ) -> Self {
        Self {
            connection_id,
            sender,
        }
    }

    /// Send a message through the channel
    pub async fn send(&self, message: BidirectionalMessage) -> Result<(), String> {
        self.sender.send(message).map_err(|e| e.to_string())
    }

    /// Get the connection ID
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }
}

/// Context information for an active WebSocket connection
#[derive(Debug, Clone)]
pub struct ConnectionContext {
    /// Unique identifier for this connection
    pub id: ConnectionId,

    /// Connection information (user, subscriptions, metadata)
    pub info: Arc<RwLock<ConnectionInfo>>,

    /// Message sender for this connection
    pub sender: ChannelMessageSender,
}

impl ConnectionContext {
    /// Create a new connection context
    pub fn new(id: ConnectionId, sender: ChannelMessageSender) -> Self {
        let info = ConnectionInfo::new(id);
        Self {
            id,
            info: Arc::new(RwLock::new(info)),
            sender,
        }
    }

    /// Check if the connection is authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.info.read().await.is_authenticated()
    }

    /// Get the authenticated user (if any)
    pub async fn get_user(&self) -> Option<Arc<AuthenticatedUser>> {
        self.info.read().await.user.clone()
    }

    /// Set the authenticated user
    pub async fn set_user(&self, user: AuthenticatedUser) {
        self.info.write().await.set_user(user);
    }

    /// Clear the authenticated user
    pub async fn clear_user(&self) {
        self.info.write().await.clear_user();
    }

    /// Check if the connection has a specific permission
    pub async fn has_permission(&self, permission: &str) -> bool {
        self.info.read().await.has_permission(permission)
    }

    /// Check if the connection is subscribed to a topic
    pub async fn is_subscribed_to(&self, topic: &str) -> bool {
        self.info.read().await.is_subscribed_to(topic)
    }

    /// Add a subscription
    pub async fn subscribe(&self, topic: String) {
        self.info.write().await.subscribe(topic);
    }

    /// Remove a subscription
    pub async fn unsubscribe(&self, topic: &str) -> bool {
        self.info.write().await.unsubscribe(topic)
    }

    /// Get all subscriptions
    pub async fn get_subscriptions(&self) -> Vec<String> {
        self.info
            .read()
            .await
            .subscriptions
            .iter()
            .cloned()
            .collect()
    }

    /// Update connection metadata
    pub async fn set_metadata(&self, key: &str, value: serde_json::Value) {
        let mut info = self.info.write().await;
        if let serde_json::Value::Object(map) = &mut info.metadata {
            map.insert(key.to_string(), value);
        }
    }

    /// Get connection metadata
    pub async fn get_metadata(&self, key: &str) -> Option<serde_json::Value> {
        let info = self.info.read().await;
        if let serde_json::Value::Object(map) = &info.metadata {
            map.get(key).cloned()
        } else {
            None
        }
    }
}
