//! WebSocket service implementation with builder pattern

use crate::{
    ConnectionContext, DefaultConnectionManager, MessageHandler, MessageRouter, ServerError,
    ServerResult, WebSocketHandler, WebSocketUpgrade, connection::ChannelMessageSender,
};
use axum::{
    extract::{State, ws::WebSocketUpgrade as AxumWebSocketUpgrade},
    http::HeaderMap,
    response::Response,
};
use bon::Builder;
use ras_auth_core::AuthProvider;
use ras_jsonrpc_bidirectional_types::{ConnectionId, ConnectionInfo, ConnectionManager};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Trait for services that handle WebSocket JSON-RPC communication
pub trait WebSocketService: Clone + Send + Sync + 'static {
    /// The message handler type
    type Handler: MessageHandler;
    /// The auth provider type
    type AuthProvider: AuthProvider;
    /// The connection manager type
    type ConnectionManager: ConnectionManager;

    /// Get the message handler
    fn handler(&self) -> Arc<Self::Handler>;

    /// Get the auth provider
    fn auth_provider(&self) -> Arc<Self::AuthProvider>;

    /// Get the connection manager
    fn connection_manager(&self) -> Arc<Self::ConnectionManager>;

    /// Check if authentication is required
    fn require_auth(&self) -> bool;

    /// Handle WebSocket upgrade
    async fn handle_upgrade(
        &self,
        upgrade: AxumWebSocketUpgrade,
        headers: HeaderMap,
    ) -> Result<Response, (axum::http::StatusCode, String)> {
        let ws_upgrade = WebSocketUpgrade::new(upgrade, headers);
        let service = self.clone();

        ws_upgrade
            .on_upgrade_with_auth(
                &*self.auth_provider(),
                self.require_auth(),
                move |socket, user| {
                    Box::pin(async move {
                        if let Err(e) = service.handle_connection(socket, user).await {
                            error!("WebSocket connection error: {}", e);
                        }
                    })
                },
            )
            .await
    }

    /// Handle an individual WebSocket connection
    fn handle_connection(
        &self,
        socket: axum::extract::ws::WebSocket,
        user: Option<ras_auth_core::AuthenticatedUser>,
    ) -> impl std::future::Future<Output = ServerResult<()>> + Send {
        let service = self.clone();
        async move {
            let connection_id = ConnectionId::new();
            info!("New WebSocket connection: {}", connection_id);

            // Create message channel for this connection
            let (message_tx, message_rx) = mpsc::unbounded_channel();
            let sender = ChannelMessageSender::new(connection_id, message_tx);

            // Create connection info and add to manager
            let mut info = ConnectionInfo::new(connection_id);
            if let Some(user) = user.clone() {
                info.set_user(user);
            }

            // Add connection to manager
            service
                .connection_manager()
                .add_connection(info)
                .await
                .map_err(|e| ServerError::ConnectionError(e))?;

            // Create connection context
            let context = Arc::new(ConnectionContext::new(connection_id, sender));
            if let Some(user) = user {
                context.set_user(user).await;
            }

            // Create and run WebSocket handler
            let handler = WebSocketHandler::new(service.handler(), context.clone(), message_rx);

            // Handle the connection (this will block until connection closes)
            let result = handler.run(socket).await;

            // Remove connection from manager
            if let Err(e) = service
                .connection_manager()
                .remove_connection(connection_id)
                .await
            {
                error!("Failed to remove connection {}: {}", connection_id, e);
            }

            result
        }
    }
}

/// Builder for creating WebSocket services
#[derive(Builder)]
pub struct WebSocketServiceBuilder<H, A, M = DefaultConnectionManager> {
    /// Message handler
    handler: Arc<H>,
    /// Auth provider
    auth_provider: Arc<A>,
    /// Connection manager
    connection_manager: Option<Arc<M>>,
    /// Whether authentication is required
    #[builder(default = false)]
    require_auth: bool,
}

impl<H, A> WebSocketServiceBuilder<H, A, DefaultConnectionManager>
where
    H: MessageHandler,
    A: AuthProvider,
{
    /// Build the WebSocket service with default connection manager
    pub fn build(self) -> BuiltWebSocketService<H, A, DefaultConnectionManager> {
        BuiltWebSocketService {
            handler: self.handler,
            auth_provider: self.auth_provider,
            connection_manager: self
                .connection_manager
                .unwrap_or_else(|| Arc::new(DefaultConnectionManager::new())),
            require_auth: self.require_auth,
        }
    }
}

impl<H, A, M> WebSocketServiceBuilder<H, A, M>
where
    H: MessageHandler,
    A: AuthProvider,
    M: ConnectionManager,
{
    /// Build the WebSocket service with custom connection manager
    pub fn build_with_manager(self, manager: Arc<M>) -> BuiltWebSocketService<H, A, M> {
        BuiltWebSocketService {
            handler: self.handler,
            auth_provider: self.auth_provider,
            connection_manager: manager,
            require_auth: self.require_auth,
        }
    }
}

/// Built WebSocket service
pub struct BuiltWebSocketService<H, A, M> {
    handler: Arc<H>,
    auth_provider: Arc<A>,
    connection_manager: Arc<M>,
    require_auth: bool,
}

impl<H, A, M> Clone for BuiltWebSocketService<H, A, M> {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            auth_provider: self.auth_provider.clone(),
            connection_manager: self.connection_manager.clone(),
            require_auth: self.require_auth,
        }
    }
}

impl<H, A, M> WebSocketService for BuiltWebSocketService<H, A, M>
where
    H: MessageHandler + 'static,
    A: AuthProvider + 'static,
    M: ConnectionManager + 'static,
{
    type Handler = H;
    type AuthProvider = A;
    type ConnectionManager = M;

    fn handler(&self) -> Arc<Self::Handler> {
        self.handler.clone()
    }

    fn auth_provider(&self) -> Arc<Self::AuthProvider> {
        self.auth_provider.clone()
    }

    fn connection_manager(&self) -> Arc<Self::ConnectionManager> {
        self.connection_manager.clone()
    }

    fn require_auth(&self) -> bool {
        self.require_auth
    }
}

/// Convenience function to create a simple router-based service
pub fn create_router_service<A>(
    router: MessageRouter,
    auth_provider: Arc<A>,
    require_auth: bool,
) -> BuiltWebSocketService<MessageRouter, A, DefaultConnectionManager>
where
    A: AuthProvider,
{
    let builder = WebSocketServiceBuilder::builder()
        .handler(Arc::new(router))
        .auth_provider(auth_provider)
        .require_auth(require_auth)
        .build();
    builder.build()
}

/// Axum handler function for WebSocket upgrade
pub async fn websocket_handler<S>(
    ws: AxumWebSocketUpgrade,
    headers: HeaderMap,
    State(service): State<S>,
) -> Result<Response, (axum::http::StatusCode, String)>
where
    S: WebSocketService,
{
    service.handle_upgrade(ws, headers).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ras_auth_core::{AuthError, AuthenticatedUser};
    use std::collections::HashSet;

    // Mock auth provider for testing
    #[derive(Clone)]
    struct MockAuthProvider;

    impl AuthProvider for MockAuthProvider {
        fn authenticate(&self, token: String) -> ras_auth_core::AuthFuture<'_> {
            Box::pin(async move {
                if token == "valid_token" {
                    Ok(AuthenticatedUser {
                        user_id: "test_user".to_string(),
                        permissions: HashSet::new(),
                        metadata: None,
                    })
                } else {
                    Err(AuthError::InvalidToken)
                }
            })
        }
    }

    #[tokio::test]
    async fn test_service_builder() {
        let router = MessageRouter::new();
        let auth_provider = Arc::new(MockAuthProvider);

        let service = create_router_service(router, auth_provider, false);

        assert!(!service.require_auth());
        assert_eq!(service.connection_manager().connection_count(), 0);
    }

    #[tokio::test]
    async fn test_service_with_auth_required() {
        let router = MessageRouter::new();
        let auth_provider = Arc::new(MockAuthProvider);

        let builder = WebSocketServiceBuilder::builder()
            .handler(Arc::new(router))
            .auth_provider(auth_provider)
            .require_auth(true)
            .build();
        let service = builder.build();

        assert!(service.require_auth());
    }
}
