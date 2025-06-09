//! Message handlers for WebSocket communication

use crate::{ConnectionContext, ServerError, ServerResult};
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::StreamExt;
use ras_jsonrpc_bidirectional_types::BidirectionalMessage;
use ras_jsonrpc_types::{JsonRpcRequest, JsonRpcResponse};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Trait for handling JSON-RPC requests within a WebSocket context
#[async_trait]
pub trait MessageHandler: Send + Sync + 'static {
    /// Handle an incoming JSON-RPC request
    ///
    /// # Arguments
    /// * `request` - The JSON-RPC request to handle
    /// * `context` - The connection context containing auth info and metadata
    ///
    /// # Returns
    /// * `Ok(Some(response))` - Response to send back to client
    /// * `Ok(None)` - No response needed (for notifications)
    /// * `Err(error)` - Error occurred during handling
    async fn handle_request(
        &self,
        request: JsonRpcRequest,
        context: Arc<ConnectionContext>,
    ) -> ServerResult<Option<JsonRpcResponse>>;

    /// Handle subscription requests
    async fn handle_subscribe(
        &self,
        topics: Vec<String>,
        context: Arc<ConnectionContext>,
    ) -> ServerResult<()> {
        // Default implementation - just subscribe to topics
        for topic in topics {
            context.subscribe(topic).await;
        }
        Ok(())
    }

    /// Handle unsubscription requests
    async fn handle_unsubscribe(
        &self,
        topics: Vec<String>,
        context: Arc<ConnectionContext>,
    ) -> ServerResult<()> {
        // Default implementation - just unsubscribe from topics
        for topic in topics {
            context.unsubscribe(&topic).await;
        }
        Ok(())
    }

    /// Handle connection established event
    async fn on_connect(&self, context: Arc<ConnectionContext>) -> ServerResult<()> {
        info!("Connection established: {}", context.id);
        Ok(())
    }

    /// Handle connection closed event
    async fn on_disconnect(
        &self,
        context: Arc<ConnectionContext>,
        reason: Option<String>,
    ) -> ServerResult<()> {
        info!("Connection closed: {} (reason: {:?})", context.id, reason);
        Ok(())
    }

    /// Handle ping message
    async fn on_ping(&self, _context: Arc<ConnectionContext>) -> ServerResult<()> {
        // Default implementation - just log
        debug!("Received ping");
        Ok(())
    }

    /// Handle pong message
    async fn on_pong(&self, _context: Arc<ConnectionContext>) -> ServerResult<()> {
        // Default implementation - just log
        debug!("Received pong");
        Ok(())
    }
}

/// WebSocket connection handler that manages the message flow
pub struct WebSocketHandler<H: MessageHandler> {
    /// The message handler for processing requests
    handler: Arc<H>,
    /// Connection context
    context: Arc<ConnectionContext>,
    /// Channel for receiving messages to send to client
    message_rx: mpsc::UnboundedReceiver<BidirectionalMessage>,
}

impl<H: MessageHandler> WebSocketHandler<H> {
    /// Create a new WebSocket handler
    pub fn new(
        handler: Arc<H>,
        context: Arc<ConnectionContext>,
        message_rx: mpsc::UnboundedReceiver<BidirectionalMessage>,
    ) -> Self {
        Self {
            handler,
            context,
            message_rx,
        }
    }

    /// Run the WebSocket handler loop
    pub async fn run(mut self, mut socket: WebSocket) -> ServerResult<()> {
        info!(
            "Starting WebSocket handler for connection: {}",
            self.context.id
        );

        // Notify handler of connection
        if let Err(e) = self.handler.on_connect(self.context.clone()).await {
            error!("Error in on_connect handler: {}", e);
        }

        // Send connection established message
        let established_msg = BidirectionalMessage::ConnectionEstablished {
            connection_id: self.context.id,
        };
        if let Err(e) = socket
            .send(Message::Text(
                serde_json::to_string(&established_msg)?.into(),
            ))
            .await
        {
            error!("Failed to send connection established message: {}", e);
        }

        // Main message handling loop
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = socket.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            if let Err(e) = self.handle_websocket_message(msg, &mut socket).await {
                                error!("Error handling WebSocket message: {}", e);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            debug!("WebSocket connection closed by client");
                            break;
                        }
                    }
                }

                // Handle outgoing messages
                msg = self.message_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = self.send_message(&mut socket, msg).await {
                                error!("Error sending message: {}", e);
                                break;
                            }
                        }
                        None => {
                            debug!("Message channel closed");
                            break;
                        }
                    }
                }
            }
        }

        // Notify handler of disconnection
        if let Err(e) = self.handler.on_disconnect(self.context.clone(), None).await {
            error!("Error in on_disconnect handler: {}", e);
        }

        // Send connection closed message
        let closed_msg = BidirectionalMessage::ConnectionClosed {
            connection_id: self.context.id,
            reason: None,
        };
        let _ = socket
            .send(Message::Text(serde_json::to_string(&closed_msg)?.into()))
            .await;

        info!(
            "WebSocket handler finished for connection: {}",
            self.context.id
        );
        Ok(())
    }

    /// Handle incoming WebSocket messages
    async fn handle_websocket_message(
        &mut self,
        msg: Message,
        socket: &mut WebSocket,
    ) -> ServerResult<()> {
        match msg {
            Message::Text(text) => {
                debug!("Received text message: {}", text);
                self.handle_text_message(text.to_string(), socket).await
            }
            Message::Binary(data) => {
                debug!("Received binary message ({} bytes)", data.len());
                // Try to parse as UTF-8 text
                match String::from_utf8(data.to_vec()) {
                    Ok(text) => self.handle_text_message(text, socket).await,
                    Err(_) => {
                        warn!("Received non-UTF-8 binary message, ignoring");
                        Ok(())
                    }
                }
            }
            Message::Ping(data) => {
                debug!("Received ping");
                socket
                    .send(Message::Pong(data))
                    .await
                    .map_err(|e| ServerError::WebSocketError(e.to_string()))?;
                self.handler.on_ping(self.context.clone()).await
            }
            Message::Pong(_) => {
                debug!("Received pong");
                self.handler.on_pong(self.context.clone()).await
            }
            Message::Close(close_frame) => {
                debug!("Received close frame: {:?}", close_frame);
                let reason = close_frame.map(|f| f.reason.to_string());
                self.handler
                    .on_disconnect(self.context.clone(), reason)
                    .await?;
                Err(ServerError::WebSocketError("Connection closed".to_string()))
            }
        }
    }

    /// Handle text messages (JSON-RPC or bidirectional messages)
    async fn handle_text_message(
        &mut self,
        text: String,
        socket: &mut WebSocket,
    ) -> ServerResult<()> {
        // Try to parse as BidirectionalMessage first
        if let Ok(msg) = serde_json::from_str::<BidirectionalMessage>(&text) {
            return self.handle_bidirectional_message(msg, socket).await;
        }

        // Try to parse as JSON-RPC request
        if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&text) {
            return self.handle_jsonrpc_request(request, socket).await;
        }

        // If neither worked, return error
        Err(ServerError::InvalidRequest(format!(
            "Could not parse message as JSON-RPC or bidirectional message: {}",
            text
        )))
    }

    /// Handle bidirectional messages
    async fn handle_bidirectional_message(
        &mut self,
        msg: BidirectionalMessage,
        _socket: &mut WebSocket,
    ) -> ServerResult<()> {
        match msg {
            BidirectionalMessage::Request(request) => {
                // Handle as JSON-RPC request
                self.handle_jsonrpc_request(request, _socket).await
            }
            BidirectionalMessage::Subscribe { topics } => {
                self.handler
                    .handle_subscribe(topics, self.context.clone())
                    .await
            }
            BidirectionalMessage::Unsubscribe { topics } => {
                self.handler
                    .handle_unsubscribe(topics, self.context.clone())
                    .await
            }
            BidirectionalMessage::Ping => self.handler.on_ping(self.context.clone()).await,
            BidirectionalMessage::Pong => self.handler.on_pong(self.context.clone()).await,
            // Other message types are typically server-to-client
            _ => {
                warn!("Received unexpected bidirectional message type from client");
                Ok(())
            }
        }
    }

    /// Handle JSON-RPC requests
    async fn handle_jsonrpc_request(
        &mut self,
        request: JsonRpcRequest,
        socket: &mut WebSocket,
    ) -> ServerResult<()> {
        debug!("Handling JSON-RPC request: {}", request.method);

        match self
            .handler
            .handle_request(request, self.context.clone())
            .await
        {
            Ok(Some(response)) => {
                // Send response back to client
                let response_msg = BidirectionalMessage::Response(response);
                self.send_message(socket, response_msg).await
            }
            Ok(None) => {
                // No response needed (notification)
                Ok(())
            }
            Err(e) => {
                error!("Error handling request: {}", e);
                // Could send error response here if needed
                Err(e)
            }
        }
    }

    /// Send a message to the WebSocket client
    async fn send_message(
        &self,
        socket: &mut WebSocket,
        msg: BidirectionalMessage,
    ) -> ServerResult<()> {
        let json = serde_json::to_string(&msg)?;
        socket
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| ServerError::WebSocketError(e.to_string()))
    }
}
