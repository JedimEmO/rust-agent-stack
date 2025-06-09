//! Server code generation for bidirectional JSON-RPC services

use crate::{AuthRequirement, BidirectionalServiceDefinition};
use quote::quote;

pub fn generate_server_code(
    service_def: &BidirectionalServiceDefinition,
) -> proc_macro2::TokenStream {
    let service_name = &service_def.service_name;
    let service_trait_name = quote::format_ident!("{}Service", service_name);
    let handler_name = quote::format_ident!("{}Handler", service_name);
    let builder_name = quote::format_ident!("{}Builder", service_name);

    // Generate trait methods for client_to_server handlers
    let trait_methods = service_def.client_to_server.iter().map(|method| {
        let method_name = &method.name;
        let request_type = &method.request_type;
        let response_type = &method.response_type;

        match &method.auth {
            AuthRequirement::Unauthorized => {
                quote! {
                    async fn #method_name(&self, request: #request_type) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>>;
                }
            }
            AuthRequirement::WithPermissions(_) => {
                quote! {
                    async fn #method_name(&self, user: &ras_auth_core::AuthenticatedUser, request: #request_type) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>>;
                }
            }
        }
    });

    // Generate notification methods that can be called on the service for server_to_client notifications
    let notification_methods = service_def.server_to_client.iter().map(|notification| {
        let notification_name = &notification.name;
        let params_type = &notification.params_type;
        let method_name = quote::format_ident!("notify_{}", notification_name);

        quote! {
            async fn #method_name(&self, connection_id: ras_jsonrpc_bidirectional_types::ConnectionId, params: #params_type) -> ras_jsonrpc_bidirectional_types::Result<()>;
        }
    });

    // Generate message handler implementation
    let request_handlers = service_def.client_to_server.iter().map(|method| {
        let method_name = &method.name;
        let method_str = method_name.to_string();
        let request_type = &method.request_type;
        let permission_groups = match &method.auth {
            AuthRequirement::Unauthorized => Vec::new(),
            AuthRequirement::WithPermissions(groups) => groups.clone(),
        };

        match &method.auth {
            AuthRequirement::Unauthorized => {
                quote! {
                    #method_str => {
                        // Parse parameters
                        let params: #request_type = if let Some(params) = request.params {
                            serde_json::from_value(params)
                                .map_err(|e| ras_jsonrpc_bidirectional_server::ServerError::InvalidRequest(format!("Invalid params: {}", e)))?
                        } else {
                            // For unit type (), we can deserialize from null
                            serde_json::from_value(serde_json::Value::Null)
                                .map_err(|e| ras_jsonrpc_bidirectional_server::ServerError::InvalidRequest(format!("Invalid params: {}", e)))?
                        };

                        // Call handler
                        match self.service.#method_name(params).await {
                            Ok(result) => {
                                let result_value = serde_json::to_value(result)
                                    .map_err(|e| ras_jsonrpc_bidirectional_server::ServerError::Internal(e.to_string()))?;
                                Ok(Some(ras_jsonrpc_types::JsonRpcResponse::success(result_value, request.id.clone())))
                            }
                            Err(e) => Err(ras_jsonrpc_bidirectional_server::ServerError::Internal(e.to_string())),
                        }
                    }
                }
            }
            AuthRequirement::WithPermissions(_) => {
                // Generate permission groups code for quote
                let permission_groups_code = if permission_groups.is_empty() {
                    quote! { Vec::<Vec<String>>::new() }
                } else {
                    let groups = permission_groups.iter().map(|group| {
                        let perms = group.iter();
                        quote! { vec![#(#perms.to_string()),*] }
                    });
                    quote! { vec![#(#groups),*] as Vec<Vec<String>> }
                };

                quote! {
                    #method_str => {
                        // Check if user is authenticated
                        let user = context.get_user().await
                            .ok_or_else(|| ras_jsonrpc_bidirectional_server::ServerError::AuthenticationFailed(ras_auth_core::AuthError::InvalidToken))?;

                        // Check permissions - AND within groups, OR between groups
                        let required_permission_groups: Vec<Vec<String>> = #permission_groups_code;
                        // Only check permissions if we have non-empty groups
                        let has_non_empty_groups = required_permission_groups.iter().any(|g| !g.is_empty());
                        if has_non_empty_groups {
                            let mut has_permission = false;

                            // Check each permission group (OR logic between groups)
                            for permission_group in &required_permission_groups {
                                // Check if user has ALL permissions in this group (AND logic within group)
                                if permission_group.is_empty() {
                                    // Empty group means any authenticated user can access
                                    has_permission = true;
                                    break;
                                } else {
                                    // Check if user has all permissions in this group
                                    let group_satisfied = permission_group.iter()
                                        .all(|perm| user.permissions.contains(perm));
                                    if group_satisfied {
                                        has_permission = true;
                                        break;
                                    }
                                }
                            }

                            if !has_permission {
                                // Find the first non-empty group for error reporting
                                let first_group = required_permission_groups.iter()
                                    .find(|g| !g.is_empty())
                                    .cloned()
                                    .unwrap_or_default();
                                let error_response = ras_jsonrpc_types::JsonRpcResponse::error(
                                    ras_jsonrpc_types::JsonRpcError::new(-32002, "Insufficient permissions".to_string(), None),
                                    request.id.clone()
                                );
                                return Ok(Some(error_response));
                            }
                        }

                        // Parse parameters
                        let params: #request_type = if let Some(params) = request.params {
                            serde_json::from_value(params)
                                .map_err(|e| ras_jsonrpc_bidirectional_server::ServerError::InvalidRequest(format!("Invalid params: {}", e)))?
                        } else {
                            // For unit type (), we can deserialize from null
                            serde_json::from_value(serde_json::Value::Null)
                                .map_err(|e| ras_jsonrpc_bidirectional_server::ServerError::InvalidRequest(format!("Invalid params: {}", e)))?
                        };

                        // Call handler
                        match self.service.#method_name(&user, params).await {
                            Ok(result) => {
                                let result_value = serde_json::to_value(result)
                                    .map_err(|e| ras_jsonrpc_bidirectional_server::ServerError::Internal(e.to_string()))?;
                                Ok(Some(ras_jsonrpc_types::JsonRpcResponse::success(result_value, request.id.clone())))
                            }
                            Err(e) => Err(ras_jsonrpc_bidirectional_server::ServerError::Internal(e.to_string())),
                        }
                    }
                }
            }
        }
    });

    // Generate default notification implementations
    let default_notification_impls = service_def.server_to_client.iter().map(|notification| {
        let notification_name = &notification.name;
        let params_type = &notification.params_type;
        let method_name = quote::format_ident!("notify_{}", notification_name);
        let notification_str = notification_name.to_string();

        quote! {
            async fn #method_name(&self, connection_id: ras_jsonrpc_bidirectional_types::ConnectionId, params: #params_type) -> ras_jsonrpc_bidirectional_types::Result<()> {
                let notification = ras_jsonrpc_bidirectional_types::ServerNotification {
                    method: #notification_str.to_string(),
                    params: serde_json::to_value(params)
                        .map_err(ras_jsonrpc_bidirectional_types::BidirectionalError::from)?,
                    metadata: None,
                };

                let message = ras_jsonrpc_bidirectional_types::BidirectionalMessage::ServerNotification(notification);
                self.connection_manager.send_to_connection(connection_id, message).await
            }
        }
    });

    quote! {
        #[cfg(feature = "server")]
        /// Generated bidirectional service trait
        #[async_trait::async_trait]
        pub trait #service_trait_name: Send + Sync + 'static {
            #(#trait_methods)*
            #(#notification_methods)*
        }

        #[cfg(feature = "server")]
        /// Generated message handler for the bidirectional service
        pub struct #handler_name<T: #service_trait_name, M: ras_jsonrpc_bidirectional_types::ConnectionManager + 'static> {
            service: std::sync::Arc<T>,
            connection_manager: std::sync::Arc<M>,
        }

        #[cfg(feature = "server")]
        impl<T: #service_trait_name, M: ras_jsonrpc_bidirectional_types::ConnectionManager + 'static> #handler_name<T, M> {
            pub fn new(
                service: std::sync::Arc<T>,
                connection_manager: std::sync::Arc<M>,
            ) -> Self {
                Self { service, connection_manager }
            }

            #(#default_notification_impls)*
        }

        #[cfg(feature = "server")]
        #[async_trait::async_trait]
        impl<T: #service_trait_name, M: ras_jsonrpc_bidirectional_types::ConnectionManager + 'static> ras_jsonrpc_bidirectional_server::MessageHandler for #handler_name<T, M> {
            async fn handle_request(
                &self,
                request: ras_jsonrpc_types::JsonRpcRequest,
                context: std::sync::Arc<ras_jsonrpc_bidirectional_server::ConnectionContext>,
            ) -> ras_jsonrpc_bidirectional_server::ServerResult<Option<ras_jsonrpc_types::JsonRpcResponse>> {
                // Dispatch method
                match request.method.as_str() {
                    #(#request_handlers)*
                    _ => Err(ras_jsonrpc_bidirectional_server::ServerError::HandlerNotFound(request.method.clone()))
                }
            }

        }

        #[cfg(feature = "server")]
        /// Builder for the bidirectional WebSocket service
        pub struct #builder_name<T: #service_trait_name, A: ras_auth_core::AuthProvider> {
            service: std::sync::Arc<T>,
            auth_provider: std::sync::Arc<A>,
            require_auth: bool,
        }

        #[cfg(feature = "server")]
        impl<T: #service_trait_name, A: ras_auth_core::AuthProvider> #builder_name<T, A> {
            /// Create a new builder
            pub fn new(service: T, auth_provider: A) -> Self {
                Self {
                    service: std::sync::Arc::new(service),
                    auth_provider: std::sync::Arc::new(auth_provider),
                    require_auth: false,
                }
            }

            /// Set whether authentication is required
            pub fn require_auth(mut self, require_auth: bool) -> Self {
                self.require_auth = require_auth;
                self
            }

            /// Build the WebSocket service
            pub fn build(self) -> ras_jsonrpc_bidirectional_server::service::BuiltWebSocketService<#handler_name<T, ras_jsonrpc_bidirectional_server::DefaultConnectionManager>, A, ras_jsonrpc_bidirectional_server::DefaultConnectionManager> {
                use ras_jsonrpc_bidirectional_server::DefaultConnectionManager;

                let connection_manager = std::sync::Arc::new(DefaultConnectionManager::new());
                let handler = #handler_name::new(
                    self.service.clone(),
                    connection_manager.clone(),
                );

                let builder = ras_jsonrpc_bidirectional_server::WebSocketServiceBuilder::builder()
                    .handler(std::sync::Arc::new(handler))
                    .auth_provider(self.auth_provider)
                    .require_auth(self.require_auth)
                    .build();
                builder.build()
            }
        }
    }
}
