//! Native WebSocket transport implementation using tokio-tungstenite

use crate::{
    WebSocketTransport,
    config::ClientConfig,
    error::{ClientError, ClientResult},
};
use async_trait::async_trait;
use futures::{FutureExt, SinkExt, StreamExt};
use ras_jsonrpc_bidirectional_types::BidirectionalMessage;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::Message,
    tungstenite::{handshake::client::generate_key, http::Request},
};
use tracing::{debug, info, warn};
use url::Url;

/// Native WebSocket transport using tokio-tungstenite
pub struct NativeWebSocketTransport {
    config: ClientConfig,
    connection: Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    url: Url,
}

impl NativeWebSocketTransport {
    /// Create a new native WebSocket transport
    pub fn new(config: ClientConfig) -> Self {
        let url =
            Url::parse(&config.get_connection_url()).expect("URL should be validated in config");

        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
            url,
        }
    }

    /// Build request headers for the WebSocket connection
    fn build_request_headers(&self) -> http::HeaderMap {
        let mut headers = http::HeaderMap::new();

        for (key, value) in self.config.get_connection_headers() {
            if let (Ok(header_name), Ok(header_value)) = (
                http::HeaderName::try_from(key),
                http::HeaderValue::try_from(value),
            ) {
                headers.insert(header_name, header_value);
            }
        }

        headers
    }
}

#[async_trait]
impl WebSocketTransport for NativeWebSocketTransport {
    async fn connect(&mut self) -> ClientResult<()> {
        info!("Connecting to WebSocket server: {}", self.url);

        // Create connection request with headers using IntoClientRequest

        // Extract host from URL for Host header
        let host = self.url.host_str().unwrap_or("localhost");
        let host_header = if let Some(port) = self.url.port() {
            format!("{}:{}", host, port)
        } else {
            host.to_string()
        };

        let mut request = Request::builder()
            .method("GET")
            .uri(self.url.as_str())
            .header("Host", host_header)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key());

        // Add custom headers from build_request_headers()
        let headers = self.build_request_headers();
        for (name, value) in headers.iter() {
            let header_name = name.as_str().to_lowercase();
            // Skip WebSocket-specific headers that are already set
            if !header_name.starts_with("sec-websocket")
                && header_name != "connection"
                && header_name != "upgrade"
                && header_name != "host"
            {
                request = request.header(name, value);
            }
        }

        let request = request
            .body(())
            .map_err(|e| ClientError::connection(format!("Failed to build request: {}", e)))?;

        // Configure connection
        let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        config.max_message_size = Some(16 * 1024 * 1024); // 16MB
        config.max_frame_size = Some(16 * 1024 * 1024); // 16MB
        config.accept_unmasked_frames = false;

        // Connect with timeout
        let connect_future = connect_async_with_config(request, Some(config), false);
        let (ws_stream, response) =
            tokio::time::timeout(self.config.connection_timeout, connect_future)
                .await
                .map_err(|_| ClientError::timeout(self.config.connection_timeout.as_secs()))?
                .map_err(|e| {
                    ClientError::connection(format!("WebSocket connection failed: {}", e))
                })?;

        debug!(
            "WebSocket connection established, status: {}",
            response.status()
        );

        // Store the connection
        *self.connection.write().await = Some(ws_stream);

        info!("Successfully connected to WebSocket server");
        Ok(())
    }

    async fn disconnect(&mut self) -> ClientResult<()> {
        info!("Disconnecting from WebSocket server");

        if let Some(mut ws) = self.connection.write().await.take() {
            // Send close frame
            if let Err(e) = ws.close(None).await {
                warn!("Error sending close frame: {}", e);
            }

            info!("WebSocket connection closed");
        }

        Ok(())
    }

    async fn send(&mut self, message: &BidirectionalMessage) -> ClientResult<()> {
        let json = serde_json::to_string(message).map_err(ClientError::Json)?;

        debug!("Sending message: {}", json);

        let ws_message = Message::Text(json.into());

        let mut connection_guard = self.connection.write().await;
        if let Some(ref mut ws) = *connection_guard {
            ws.send(ws_message)
                .await
                .map_err(|e| ClientError::send_failed(format!("Failed to send message: {}", e)))?;
            Ok(())
        } else {
            Err(ClientError::NotConnected)
        }
    }

    async fn receive(&mut self) -> ClientResult<Option<BidirectionalMessage>> {
        let mut connection_guard = self.connection.write().await;
        if let Some(ref mut ws) = *connection_guard {
            // Try to receive a message (non-blocking)
            match ws.next().now_or_never() {
                Some(Some(message)) => {
                    let message = message.map_err(|e| {
                        ClientError::receive_failed(format!("WebSocket error: {}", e))
                    })?;

                    match message {
                        Message::Text(text) => {
                            debug!("Received text message: {}", text);
                            let bidirectional_message: BidirectionalMessage =
                                serde_json::from_str(&text).map_err(ClientError::Json)?;
                            Ok(Some(bidirectional_message))
                        }
                        Message::Binary(data) => {
                            debug!("Received binary message ({} bytes)", data.len());
                            let bidirectional_message: BidirectionalMessage =
                                serde_json::from_slice(&data).map_err(ClientError::Json)?;
                            Ok(Some(bidirectional_message))
                        }
                        Message::Close(close_frame) => {
                            info!("Received close frame: {:?}", close_frame);
                            *self.connection.write().await = None;
                            Err(ClientError::connection("Connection closed by server"))
                        }
                        Message::Ping(data) => {
                            debug!("Received ping, sending pong");
                            if let Err(e) = ws.send(Message::Pong(data)).await {
                                warn!("Failed to send pong: {}", e);
                            }
                            Ok(None) // No message to return
                        }
                        Message::Pong(_) => {
                            debug!("Received pong");
                            Ok(None) // No message to return
                        }
                        Message::Frame(_) => {
                            // Raw frames are not expected in normal operation
                            warn!("Received unexpected raw frame");
                            Ok(None)
                        }
                    }
                }
                Some(None) => {
                    // Stream ended
                    info!("WebSocket stream ended");
                    *self.connection.write().await = None;
                    Err(ClientError::connection("WebSocket stream ended"))
                }
                None => {
                    // No message available right now
                    Ok(None)
                }
            }
        } else {
            Err(ClientError::NotConnected)
        }
    }

    fn is_connected(&self) -> bool {
        // We can't easily check this without potentially blocking,
        // so we'll check if we have a connection stored
        futures::executor::block_on(async { self.connection.read().await.is_some() })
    }

    fn url(&self) -> &str {
        self.url.as_str()
    }
}

impl std::fmt::Debug for NativeWebSocketTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeWebSocketTransport")
            .field("url", &self.url.as_str())
            .field("is_connected", &self.is_connected())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClientConfig;

    #[test]
    fn test_native_transport_creation() {
        let config = ClientConfig::new("ws://localhost:8080/ws");
        let transport = NativeWebSocketTransport::new(config);

        assert_eq!(transport.url(), "ws://localhost:8080/ws");
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_build_request_headers() {
        let mut config = ClientConfig::new("ws://localhost:8080/ws");
        config
            .custom_headers
            .insert("X-Custom".to_string(), "value".to_string());

        let transport = NativeWebSocketTransport::new(config);
        let headers = transport.build_request_headers();

        assert!(headers.contains_key("X-Custom"));
    }

    #[tokio::test]
    async fn test_disconnect_without_connection() {
        let config = ClientConfig::new("ws://localhost:8080/ws");
        let mut transport = NativeWebSocketTransport::new(config);

        // Should not error when disconnecting without being connected
        let result = transport.disconnect().await;
        assert!(result.is_ok());
    }
}
