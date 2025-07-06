//! Message sender trait for bidirectional JSON-RPC

use crate::{BidirectionalError, BidirectionalMessage, ConnectionId, Result};
use async_trait::async_trait;
use futures::sink::SinkExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Trait for sending messages over WebSocket connections
#[async_trait]
pub trait MessageSender: Send + Sync {
    /// Send a message to a WebSocket connection
    async fn send_message(&self, message: BidirectionalMessage) -> Result<()>;

    /// Close the connection
    async fn close(&self) -> Result<()>;

    /// Check if the connection is still open
    async fn is_connected(&self) -> bool;

    /// Get the connection ID
    fn connection_id(&self) -> ConnectionId;
}

/// A message sender implementation using tokio-tungstenite
pub struct WebSocketMessageSender<S>
where
    S: SinkExt<WsMessage> + Send + Unpin,
{
    connection_id: ConnectionId,
    sink: Arc<Mutex<S>>,
    is_closed: Arc<Mutex<bool>>,
}

impl<S> WebSocketMessageSender<S>
where
    S: SinkExt<WsMessage> + Send + Unpin,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    /// Create a new WebSocket message sender
    pub fn new(connection_id: ConnectionId, sink: S) -> Self {
        Self {
            connection_id,
            sink: Arc::new(Mutex::new(sink)),
            is_closed: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl<S> MessageSender for WebSocketMessageSender<S>
where
    S: SinkExt<WsMessage> + Send + Unpin,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    async fn send_message(&self, message: BidirectionalMessage) -> Result<()> {
        if self.is_connected().await {
            let json = serde_json::to_string(&message)?;
            let ws_message = WsMessage::Text(json.into());

            let mut sink = self.sink.lock().await;
            sink.send(ws_message)
                .await
                .map_err(|e| BidirectionalError::SendError(e.to_string()))?;

            Ok(())
        } else {
            Err(BidirectionalError::ConnectionClosed)
        }
    }

    async fn close(&self) -> Result<()> {
        let mut is_closed = self.is_closed.lock().await;
        if !*is_closed {
            *is_closed = true;

            let mut sink = self.sink.lock().await;
            sink.send(WsMessage::Close(None))
                .await
                .map_err(|e| BidirectionalError::SendError(e.to_string()))?;
        }
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        !*self.is_closed.lock().await
    }

    fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }
}

/// Extension trait for message senders with convenience methods
#[async_trait]
pub trait MessageSenderExt: MessageSender {
    /// Send a JSON-RPC request
    async fn send_request(&self, request: ras_jsonrpc_types::JsonRpcRequest) -> Result<()> {
        self.send_message(BidirectionalMessage::Request(request))
            .await
    }

    /// Send a JSON-RPC response
    async fn send_response(&self, response: ras_jsonrpc_types::JsonRpcResponse) -> Result<()> {
        self.send_message(BidirectionalMessage::Response(response))
            .await
    }

    /// Send a server notification
    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        let notification = crate::ServerNotification {
            method: method.to_string(),
            params,
            metadata: None,
        };
        self.send_message(BidirectionalMessage::ServerNotification(notification))
            .await
    }

    /// Send a ping message
    async fn send_ping(&self) -> Result<()> {
        self.send_message(BidirectionalMessage::Ping).await
    }

    /// Send a pong message
    async fn send_pong(&self) -> Result<()> {
        self.send_message(BidirectionalMessage::Pong).await
    }

    /// Send a subscription confirmation
    async fn send_subscription_update(&self, topics: Vec<String>, subscribed: bool) -> Result<()> {
        let message = if subscribed {
            BidirectionalMessage::Subscribe { topics }
        } else {
            BidirectionalMessage::Unsubscribe { topics }
        };
        self.send_message(message).await
    }
}

// Blanket implementation for all MessageSender types
impl<T: MessageSender> MessageSenderExt for T {}

/// A no-operation message sender that does nothing
pub struct NoOpMessageSender {
    connection_id: ConnectionId,
}

impl NoOpMessageSender {
    /// Create a new no-op message sender
    pub fn new() -> Self {
        Self {
            connection_id: ConnectionId::new(),
        }
    }

    /// Create a new no-op message sender with a specific connection ID
    pub fn with_connection_id(connection_id: ConnectionId) -> Self {
        Self { connection_id }
    }
}

impl Default for NoOpMessageSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageSender for NoOpMessageSender {
    async fn send_message(&self, _message: BidirectionalMessage) -> Result<()> {
        // No-op implementation - just return success
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // No-op implementation - just return success
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        // Always report as connected for testing purposes
        true
    }

    fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_sender_ext() {
        // Create a mock sender
        struct MockSender {
            connection_id: ConnectionId,
            sent_messages: Arc<Mutex<Vec<BidirectionalMessage>>>,
        }

        #[async_trait]
        impl MessageSender for MockSender {
            async fn send_message(&self, message: BidirectionalMessage) -> Result<()> {
                self.sent_messages.lock().await.push(message);
                Ok(())
            }

            async fn close(&self) -> Result<()> {
                Ok(())
            }

            async fn is_connected(&self) -> bool {
                true
            }

            fn connection_id(&self) -> ConnectionId {
                self.connection_id
            }
        }

        let sender = MockSender {
            connection_id: ConnectionId::new(),
            sent_messages: Arc::new(Mutex::new(Vec::new())),
        };

        // Test convenience methods
        sender.send_ping().await.unwrap();
        sender.send_pong().await.unwrap();
        sender
            .send_notification("test.method", serde_json::json!({"key": "value"}))
            .await
            .unwrap();

        let messages = sender.sent_messages.lock().await;
        assert_eq!(messages.len(), 3);

        // Check message types
        assert!(matches!(messages[0], BidirectionalMessage::Ping));
        assert!(matches!(messages[1], BidirectionalMessage::Pong));
        assert!(matches!(
            &messages[2],
            BidirectionalMessage::ServerNotification(n) if n.method == "test.method"
        ));
    }
}
