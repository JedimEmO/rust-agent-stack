//! Connection manager trait for bidirectional JSON-RPC

use crate::{BidirectionalMessage, ConnectionId, ConnectionInfo, Result};
use async_trait::async_trait;
use ras_auth_core::AuthenticatedUser;

/// Trait for managing WebSocket connections
#[async_trait]
pub trait ConnectionManager: Send + Sync {
    /// Add a new connection
    async fn add_connection(&self, info: ConnectionInfo) -> Result<()>;

    /// Remove a connection
    async fn remove_connection(&self, id: ConnectionId) -> Result<()>;

    /// Get connection information
    async fn get_connection(&self, id: ConnectionId) -> Result<Option<ConnectionInfo>>;

    /// Get all active connections
    async fn get_all_connections(&self) -> Result<Vec<ConnectionInfo>>;

    /// Get connections subscribed to a topic
    async fn get_subscribed_connections(&self, topic: &str) -> Result<Vec<ConnectionInfo>>;

    /// Update connection authentication
    async fn set_connection_user(&self, id: ConnectionId, user: AuthenticatedUser) -> Result<()>;

    /// Clear connection authentication
    async fn clear_connection_user(&self, id: ConnectionId) -> Result<()>;

    /// Add subscription to a connection
    async fn add_subscription(&self, id: ConnectionId, topic: String) -> Result<()>;

    /// Remove subscription from a connection
    async fn remove_subscription(&self, id: ConnectionId, topic: &str) -> Result<()>;

    /// Get all subscriptions for a connection
    async fn get_subscriptions(&self, id: ConnectionId) -> Result<Vec<String>>;

    /// Send a message to a specific connection
    async fn send_to_connection(
        &self,
        id: ConnectionId,
        message: BidirectionalMessage,
    ) -> Result<()>;

    /// Broadcast a message to all connections subscribed to a topic
    async fn broadcast_to_topic(&self, topic: &str, message: BidirectionalMessage)
    -> Result<usize>;

    /// Broadcast a message to all authenticated connections
    async fn broadcast_to_authenticated(&self, message: BidirectionalMessage) -> Result<usize>;

    /// Broadcast a message to all connections with a specific permission
    async fn broadcast_to_permission(
        &self,
        permission: &str,
        message: BidirectionalMessage,
    ) -> Result<usize>;

    /// Check if a connection exists
    async fn connection_exists(&self, id: ConnectionId) -> Result<bool> {
        Ok(self.get_connection(id).await?.is_some())
    }

    /// Get the number of active connections
    async fn connection_count(&self) -> Result<usize> {
        Ok(self.get_all_connections().await?.len())
    }

    /// Get the number of authenticated connections
    async fn authenticated_connection_count(&self) -> Result<usize> {
        let connections = self.get_all_connections().await?;
        Ok(connections.iter().filter(|c| c.is_authenticated()).count())
    }

    /// Clean up stale connections (optional implementation)
    async fn cleanup_stale_connections(&self) -> Result<usize> {
        Ok(0) // Default: no cleanup
    }
}

/// Extension trait for connection managers with convenience methods
#[async_trait]
pub trait ConnectionManagerExt: ConnectionManager {
    /// Send a JSON-RPC notification to a connection
    async fn notify_connection(
        &self,
        id: ConnectionId,
        method: &str,
        params: serde_json::Value,
    ) -> Result<()> {
        let message = BidirectionalMessage::ServerNotification(crate::ServerNotification {
            method: method.to_string(),
            params,
            metadata: None,
        });
        self.send_to_connection(id, message).await
    }

    /// Broadcast a notification to a topic
    async fn notify_topic(
        &self,
        topic: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<usize> {
        let message = BidirectionalMessage::Broadcast(crate::BroadcastMessage {
            topic: topic.to_string(),
            method: method.to_string(),
            params,
            metadata: None,
        });
        self.broadcast_to_topic(topic, message).await
    }

    /// Send a ping to check if connection is alive
    async fn ping_connection(&self, id: ConnectionId) -> Result<()> {
        self.send_to_connection(id, BidirectionalMessage::Ping)
            .await
    }

    /// Get connections by user ID
    async fn get_user_connections(&self, user_id: &str) -> Result<Vec<ConnectionInfo>> {
        let all = self.get_all_connections().await?;
        Ok(all
            .into_iter()
            .filter(|c| {
                c.user
                    .as_ref()
                    .map(|u| u.user_id == user_id)
                    .unwrap_or(false)
            })
            .collect())
    }

    /// Disconnect all connections for a user
    async fn disconnect_user(&self, user_id: &str) -> Result<usize> {
        let connections = self.get_user_connections(user_id).await?;
        let count = connections.len();

        for conn in connections {
            if let Err(e) = self.remove_connection(conn.id).await {
                tracing::error!("Failed to disconnect user connection {}: {}", conn.id, e);
            }
        }

        Ok(count)
    }
}

// Blanket implementation for all ConnectionManager types
impl<T: ConnectionManager> ConnectionManagerExt for T {}
