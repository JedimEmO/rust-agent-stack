use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Token, Type, parse::Parse, parse_macro_input};

mod client;
mod openrpc;
mod static_hosting;

/// Macro to generate a JSON-RPC service with authentication support
///
/// This macro generates a service trait and builder that integrates with axum
/// for handling JSON-RPC requests with authentication and authorization.
///
/// See the tests for usage examples.
#[proc_macro]
pub fn jsonrpc_service(input: TokenStream) -> TokenStream {
    let service_definition = parse_macro_input!(input as ServiceDefinition);

    match generate_service_code(service_definition) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[derive(Debug)]
struct ServiceDefinition {
    service_name: Ident,
    openrpc: Option<OpenRpcConfig>,
    explorer: Option<ExplorerConfig>,
    methods: Vec<MethodDefinition>,
}

#[derive(Debug)]
enum OpenRpcConfig {
    Enabled,
    WithPath(String),
}

#[derive(Debug)]
enum ExplorerConfig {
    Enabled,
    WithPath(String),
}

#[derive(Debug)]
struct MethodDefinition {
    auth: AuthRequirement,
    name: Ident,
    request_type: Type,
    response_type: Type,
}

#[derive(Debug)]
enum AuthRequirement {
    Unauthorized,
    WithPermissions(Vec<Vec<String>>), // Vec of permission groups - OR between groups, AND within groups
}

impl Parse for ServiceDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse the opening brace
        let content;
        syn::braced!(content in input);

        // Parse service_name: Ident
        let _ = content.parse::<Ident>()?; // "service_name"
        let _ = content.parse::<Token![:]>()?;
        let service_name = content.parse::<Ident>()?;
        let _ = content.parse::<Token![,]>()?;

        // Check if openrpc field is present
        let mut openrpc = None;
        let mut explorer = None;

        // Parse optional fields until we hit "methods"
        while content.peek(Ident) {
            let field_name = content.fork().parse::<Ident>()?;
            if field_name == "methods" {
                break;
            }

            let _ = content.parse::<Ident>()?; // field name
            let _ = content.parse::<Token![:]>()?;

            if field_name == "openrpc" {
                // Parse openrpc value - can be true/false or { output: "path" }
                if content.peek(syn::LitBool) {
                    let enabled = content.parse::<syn::LitBool>()?;
                    if enabled.value() {
                        openrpc = Some(OpenRpcConfig::Enabled);
                    }
                } else if content.peek(syn::token::Brace) {
                    let openrpc_content;
                    syn::braced!(openrpc_content in content);

                    // Parse output: "path"
                    let _ = openrpc_content.parse::<Ident>()?; // "output"
                    let _ = openrpc_content.parse::<Token![:]>()?;
                    let path = openrpc_content.parse::<LitStr>()?;
                    openrpc = Some(OpenRpcConfig::WithPath(path.value()));
                }
            } else if field_name == "explorer" {
                // Parse explorer value - can be true/false or { path: "/custom-path" }
                if content.peek(syn::LitBool) {
                    let enabled = content.parse::<syn::LitBool>()?;
                    if enabled.value() {
                        explorer = Some(ExplorerConfig::Enabled);
                    }
                } else if content.peek(syn::token::Brace) {
                    let explorer_content;
                    syn::braced!(explorer_content in content);

                    // Parse path: "/custom-path"
                    let _ = explorer_content.parse::<Ident>()?; // "path"
                    let _ = explorer_content.parse::<Token![:]>()?;
                    let path = explorer_content.parse::<LitStr>()?;
                    explorer = Some(ExplorerConfig::WithPath(path.value()));
                }
            }

            let _ = content.parse::<Token![,]>()?;
        }

        // Parse methods: [...]
        let _ = content.parse::<Ident>()?; // "methods"
        let _ = content.parse::<Token![:]>()?;

        let methods_content;
        syn::bracketed!(methods_content in content);

        let mut methods = Vec::new();
        while !methods_content.is_empty() {
            let method = methods_content.parse::<MethodDefinition>()?;
            methods.push(method);

            // Handle optional trailing comma
            if methods_content.peek(Token![,]) {
                let _ = methods_content.parse::<Token![,]>()?;
            }
        }

        Ok(ServiceDefinition {
            service_name,
            openrpc,
            explorer,
            methods,
        })
    }
}

impl Parse for MethodDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse auth requirement (UNAUTHORIZED or WITH_PERMISSIONS([...]))
        let auth = if input.peek(syn::Ident) {
            let auth_ident = input.parse::<Ident>()?;
            match auth_ident.to_string().as_str() {
                "UNAUTHORIZED" => AuthRequirement::Unauthorized,
                "WITH_PERMISSIONS" => {
                    // Parse ([...] | [...] | ...)
                    let perms_content;
                    syn::parenthesized!(perms_content in input);

                    let mut permission_groups = Vec::new();

                    // Parse first permission group
                    let first_group_content;
                    syn::bracketed!(first_group_content in perms_content);

                    let mut first_group = Vec::new();
                    while !first_group_content.is_empty() {
                        let perm = first_group_content.parse::<LitStr>()?;
                        first_group.push(perm.value());

                        if first_group_content.peek(Token![,]) {
                            let _ = first_group_content.parse::<Token![,]>()?;
                        }
                    }
                    permission_groups.push(first_group);

                    // Parse additional permission groups separated by |
                    while perms_content.peek(Token![|]) {
                        let _ = perms_content.parse::<Token![|]>()?;

                        let group_content;
                        syn::bracketed!(group_content in perms_content);

                        let mut group = Vec::new();
                        while !group_content.is_empty() {
                            let perm = group_content.parse::<LitStr>()?;
                            group.push(perm.value());

                            if group_content.peek(Token![,]) {
                                let _ = group_content.parse::<Token![,]>()?;
                            }
                        }
                        permission_groups.push(group);
                    }

                    AuthRequirement::WithPermissions(permission_groups)
                }
                _ => {
                    return Err(syn::Error::new(
                        auth_ident.span(),
                        "Expected UNAUTHORIZED or WITH_PERMISSIONS",
                    ));
                }
            }
        } else {
            return Err(syn::Error::new(
                input.span(),
                "Expected authentication requirement",
            ));
        };

        // Parse method name
        let name = input.parse::<Ident>()?;

        // Parse (RequestType)
        let request_content;
        syn::parenthesized!(request_content in input);
        let request_type = request_content.parse::<Type>()?;

        // Parse -> ResponseType
        let _ = input.parse::<Token![->]>()?;
        let response_type = input.parse::<Type>()?;

        Ok(MethodDefinition {
            auth,
            name,
            request_type,
            response_type,
        })
    }
}

fn generate_service_code(service_def: ServiceDefinition) -> syn::Result<proc_macro2::TokenStream> {
    // Generate OpenRPC code if enabled in the macro input
    let (openrpc_code, schema_checks) = if service_def.openrpc.is_some() {
        let openrpc_config = service_def.openrpc.as_ref().unwrap();
        (
            openrpc::generate_openrpc_code(&service_def, openrpc_config),
            openrpc::generate_schema_impl_checks(&service_def),
        )
    } else {
        (quote! {}, quote! {})
    };

    // Generate server code only if server feature is enabled in the macro crate
    let server_code = if cfg!(feature = "server") {
        let server_impl = generate_server_code(&service_def);

        // Generate explorer code if enabled - only if server feature is enabled
        let explorer_code = if service_def.explorer.is_some() && service_def.openrpc.is_some() {
            let explorer_config = match &service_def.explorer {
                Some(ExplorerConfig::Enabled) => static_hosting::StaticHostingConfig {
                    serve_explorer: true,
                    explorer_path: "/explorer".to_string(),
                },
                Some(ExplorerConfig::WithPath(path)) => static_hosting::StaticHostingConfig {
                    serve_explorer: true,
                    explorer_path: path.clone(),
                },
                None => static_hosting::StaticHostingConfig::default(),
            };

            // Extract base path from server code (we'll need to pass this to explorer)
            // For now, use the default empty string since JSON-RPC typically uses a single endpoint
            static_hosting::generate_static_hosting_code(
                &explorer_config,
                &service_def.service_name,
                "",
            )
        } else {
            quote! {}
        };

        // Wrap all server code in a cfg attribute to ensure it's only compiled when server feature is enabled
        quote! {
            #[cfg(feature = "server")]
            mod _generated_server {
                use super::*;

                #server_impl
                #explorer_code
            }

            #[cfg(feature = "server")]
            pub use _generated_server::*;
        }
    } else {
        quote! {}
    };

    // Generate client code only if client feature is enabled in the macro crate
    let client_code = if cfg!(feature = "client") {
        let client_impl = crate::client::generate_client_code(&service_def);

        // Wrap all client code in a cfg attribute to ensure it's only compiled when client feature is enabled
        quote! {
            #[cfg(feature = "client")]
            mod _generated_client {
                use super::*;

                #client_impl
            }

            #[cfg(feature = "client")]
            pub use _generated_client::*;
        }
    } else {
        quote! {}
    };

    let output = quote! {
        #openrpc_code
        #schema_checks
        #server_code
        #client_code
    };

    Ok(output)
}

fn generate_server_code(service_def: &ServiceDefinition) -> proc_macro2::TokenStream {
    let service_name = &service_def.service_name;
    let service_trait_name = quote::format_ident!("{}Trait", service_name);
    let builder_name = quote::format_ident!("{}Builder", service_name);

    // Generate explorer route integration if enabled
    let explorer_route_integration =
        if service_def.explorer.is_some() && service_def.openrpc.is_some() {
            let service_name_str = service_name.to_string();
            let service_name_lower = service_name_str.to_lowercase();
            let explorer_routes_fn_str = [&service_name_lower, "_explorer_routes"].concat();
            let explorer_routes_fn = syn::Ident::new(&explorer_routes_fn_str, service_name.span());
            quote! { router = router.merge(#explorer_routes_fn(&base_url)); }
        } else {
            quote! {}
        };

    // Generate trait methods
    let trait_methods = service_def.methods.iter().map(|method| {
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
                    async fn #method_name(&self, user: &ras_jsonrpc_core::AuthenticatedUser, request: #request_type) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>>;
                }
            }
        }
    });

    // Generate builder struct and implementation
    let builder_fields = service_def.methods.iter().map(|method| {
        let method_name = &method.name;
        let field_name = quote::format_ident!("{}_handler", method_name);
        let request_type = &method.request_type;
        let response_type = &method.response_type;

        match &method.auth {
            AuthRequirement::Unauthorized => {
                quote! {
                    #field_name: Option<Box<dyn Fn(#request_type) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<#response_type, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>>,
                }
            }
            AuthRequirement::WithPermissions(_) => {
                quote! {
                    #field_name: Option<Box<dyn Fn(ras_jsonrpc_core::AuthenticatedUser, #request_type) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<#response_type, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>>,
                }
            }
        }
    });

    let builder_setters = service_def.methods.iter().map(|method| {
        let method_name = &method.name;
        let setter_name = quote::format_ident!("{}_handler", method_name);
        let field_name = quote::format_ident!("{}_handler", method_name);
        let request_type = &method.request_type;
        let response_type = &method.response_type;

        match &method.auth {
            AuthRequirement::Unauthorized => {
                quote! {
                    pub fn #setter_name<F, Fut>(mut self, handler: F) -> Self
                    where
                        F: Fn(#request_type) -> Fut + Send + Sync + 'static,
                        Fut: std::future::Future<Output = Result<#response_type, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
                    {
                        self.#field_name = Some(Box::new(move |req| Box::pin(handler(req))));
                        self
                    }
                }
            }
            AuthRequirement::WithPermissions(_) => {
                quote! {
                    pub fn #setter_name<F, Fut>(mut self, handler: F) -> Self
                    where
                        F: Fn(ras_jsonrpc_core::AuthenticatedUser, #request_type) -> Fut + Send + Sync + 'static,
                        Fut: std::future::Future<Output = Result<#response_type, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
                    {
                        self.#field_name = Some(Box::new(move |user, req| Box::pin(handler(user, req))));
                        self
                    }
                }
            }
        }
    });

    // Generate field initializations for the constructor
    let field_inits = service_def.methods.iter().map(|method| {
        let field_name = quote::format_ident!("{}_handler", method.name);
        quote! { #field_name: None }
    });

    // Generate handler validation checks for the build method
    let handler_validations = service_def.methods.iter().map(|method| {
        let method_name = &method.name;
        let field_name = quote::format_ident!("{}_handler", method_name);
        let method_str = method_name.to_string();

        quote! {
            if self.#field_name.is_none() {
                missing_handlers.push(#method_str);
            }
        }
    });

    // Generate method dispatch logic for the JSON-RPC handler
    let method_dispatch = service_def.methods.iter().map(|method| {
        let method_name = &method.name;
        let method_str = method_name.to_string();
        let field_name = quote::format_ident!("{}_handler", method_name);
        let request_type = &method.request_type;
        let permission_groups = match &method.auth {
            AuthRequirement::Unauthorized => Vec::new(),
            AuthRequirement::WithPermissions(groups) => groups.clone(),
        };

        match &method.auth {
            AuthRequirement::Unauthorized => {
                quote! {
                    #method_str => {
                        if let Some(handler) = &self.#field_name {
                            // Parse parameters
                            let params: #request_type = match request.params {
                                Some(params) => match serde_json::from_value(params) {
                                    Ok(p) => p,
                                    Err(e) => return ras_jsonrpc_types::JsonRpcResponse::error(
                                        ras_jsonrpc_types::JsonRpcError::invalid_params(e.to_string()),
                                        request.id.clone()
                                    ),
                                },
                                None => match serde_json::from_value(serde_json::Value::Null) {
                                    Ok(p) => p,
                                    Err(e) => return ras_jsonrpc_types::JsonRpcResponse::error(
                                        ras_jsonrpc_types::JsonRpcError::invalid_params(e.to_string()),
                                        request.id.clone()
                                    ),
                                }
                            };

                            // Call handler with duration tracking
                            let start_time = std::time::Instant::now();
                            let handler_result = handler(params).await;
                            let duration = start_time.elapsed();

                            // Track method duration if configured
                            if let Some(duration_tracker) = &self.method_duration_tracker {
                                duration_tracker(#method_str, None, duration).await;
                            }

                            match handler_result {
                                Ok(result) => {
                                    match serde_json::to_value(result) {
                                        Ok(result_value) => ras_jsonrpc_types::JsonRpcResponse::success(result_value, request.id.clone()),
                                        Err(e) => ras_jsonrpc_types::JsonRpcResponse::error(
                                            ras_jsonrpc_types::JsonRpcError::internal_error(e.to_string()),
                                            request.id.clone()
                                        ),
                                    }
                                }
                                Err(e) => ras_jsonrpc_types::JsonRpcResponse::error(
                                    ras_jsonrpc_types::JsonRpcError::internal_error(e.to_string()),
                                    request.id.clone()
                                ),
                            }
                        } else {
                            ras_jsonrpc_types::JsonRpcResponse::error(
                                ras_jsonrpc_types::JsonRpcError::method_not_found(&#method_str),
                                request.id.clone()
                            )
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
                        if let Some(handler) = &self.#field_name {
                            // Check if user is authenticated
                            let user = match &authenticated_user {
                                Some(u) => u,
                                None => return ras_jsonrpc_types::JsonRpcResponse::error(
                                    ras_jsonrpc_types::JsonRpcError::authentication_required(),
                                    request.id.clone()
                                ),
                            };

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
                                        let group_result = self.auth_provider
                                            .as_ref()
                                            .unwrap()
                                            .check_permissions(user, permission_group);
                                        if group_result.is_ok() {
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
                                    return ras_jsonrpc_types::JsonRpcResponse::error(
                                        ras_jsonrpc_types::JsonRpcError::insufficient_permissions(
                                            first_group,
                                            user.permissions.iter().cloned().collect()
                                        ),
                                        request.id.clone()
                                    );
                                }
                            }

                            // Parse parameters
                            let params: #request_type = match request.params {
                                Some(params) => match serde_json::from_value(params) {
                                    Ok(p) => p,
                                    Err(e) => return ras_jsonrpc_types::JsonRpcResponse::error(
                                        ras_jsonrpc_types::JsonRpcError::invalid_params(e.to_string()),
                                        request.id.clone()
                                    ),
                                },
                                None => match serde_json::from_value(serde_json::Value::Null) {
                                    Ok(p) => p,
                                    Err(e) => return ras_jsonrpc_types::JsonRpcResponse::error(
                                        ras_jsonrpc_types::JsonRpcError::invalid_params(e.to_string()),
                                        request.id.clone()
                                    ),
                                }
                            };

                            // Call handler with duration tracking
                            let start_time = std::time::Instant::now();
                            let handler_result = handler(user.clone(), params).await;
                            let duration = start_time.elapsed();

                            // Track method duration if configured
                            if let Some(duration_tracker) = &self.method_duration_tracker {
                                duration_tracker(#method_str, Some(user), duration).await;
                            }

                            match handler_result {
                                Ok(result) => {
                                    match serde_json::to_value(result) {
                                        Ok(result_value) => ras_jsonrpc_types::JsonRpcResponse::success(result_value, request.id.clone()),
                                        Err(e) => ras_jsonrpc_types::JsonRpcResponse::error(
                                            ras_jsonrpc_types::JsonRpcError::internal_error(e.to_string()),
                                            request.id.clone()
                                        ),
                                    }
                                }
                                Err(e) => ras_jsonrpc_types::JsonRpcResponse::error(
                                    ras_jsonrpc_types::JsonRpcError::internal_error(e.to_string()),
                                    request.id.clone()
                                ),
                            }
                        } else {
                            ras_jsonrpc_types::JsonRpcResponse::error(
                                ras_jsonrpc_types::JsonRpcError::method_not_found(&#method_str),
                                request.id.clone()
                            )
                        }
                    }
                }
            }
        }
    });

    quote! {
        /// Generated service trait
        pub trait #service_trait_name {
            #(#trait_methods)*
        }

        /// Generated builder for the JSON-RPC service
        pub struct #builder_name {
            base_url: String,
            auth_provider: Option<Box<dyn ras_jsonrpc_core::AuthProvider>>,
            usage_tracker: Option<Box<dyn Fn(&axum::http::HeaderMap, Option<&ras_jsonrpc_core::AuthenticatedUser>, &ras_jsonrpc_types::JsonRpcRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>>,
            method_duration_tracker: Option<Box<dyn Fn(&str, Option<&ras_jsonrpc_core::AuthenticatedUser>, std::time::Duration) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>>,
            #(#builder_fields)*
        }

        impl #builder_name {
            /// Create a new builder with the base URL
            pub fn new(base_url: impl Into<String>) -> Self {
                Self {
                    base_url: base_url.into(),
                    auth_provider: None,
                    usage_tracker: None,
                    method_duration_tracker: None,
                    #(#field_inits,)*
                }
            }

            /// Set the auth provider
            pub fn auth_provider<T: ras_jsonrpc_core::AuthProvider>(mut self, provider: T) -> Self {
                self.auth_provider = Some(Box::new(provider));
                self
            }

            /// Set the usage tracker function
            /// This function will be called for each request with headers, authenticated user (if any), and the JSON-RPC request
            pub fn with_usage_tracker<F, Fut>(mut self, tracker: F) -> Self
            where
                F: Fn(&axum::http::HeaderMap, Option<&ras_jsonrpc_core::AuthenticatedUser>, &ras_jsonrpc_types::JsonRpcRequest) -> Fut + Send + Sync + 'static,
                Fut: std::future::Future<Output = ()> + Send + 'static,
            {
                self.usage_tracker = Some(Box::new(move |headers, user, request| {
                    Box::pin(tracker(headers, user, request))
                }));
                self
            }

            /// Set the method duration tracker function
            /// This function will be called after each method completes with the method name, authenticated user (if any), and the duration
            pub fn with_method_duration_tracker<F, Fut>(mut self, tracker: F) -> Self
            where
                F: Fn(&str, Option<&ras_jsonrpc_core::AuthenticatedUser>, std::time::Duration) -> Fut + Send + Sync + 'static,
                Fut: std::future::Future<Output = ()> + Send + 'static,
            {
                self.method_duration_tracker = Some(Box::new(move |method, user, duration| {
                    Box::pin(tracker(method, user, duration))
                }));
                self
            }

            #(#builder_setters)*

            /// Build the axum router for the JSON-RPC service
            pub fn build(self) -> Result<axum::Router, String> {
                // Validate that all handlers are configured
                let mut missing_handlers = Vec::new();
                #(#handler_validations)*

                if !missing_handlers.is_empty() {
                    return Err(format!(
                        "Cannot build service: the following handlers are not configured: {}",
                        missing_handlers.join(", ")
                    ));
                }

                let base_url = self.base_url.clone();
                let service = std::sync::Arc::new(self);

                let rpc_handler = axum::routing::post(move |headers: axum::http::HeaderMap, body: String| {
                    let service = service.clone();
                    async move {
                        let response = service.handle_request(headers, body).await;

                        // Determine HTTP status code based on JSON-RPC error code
                        // Map authentication/authorization errors to appropriate HTTP status codes
                        // while maintaining JSON-RPC protocol compatibility
                        let status_code = if let Some(ref error) = response.error {
                            match error.code {
                                ras_jsonrpc_types::error_codes::AUTHENTICATION_REQUIRED => axum::http::StatusCode::UNAUTHORIZED,
                                ras_jsonrpc_types::error_codes::INSUFFICIENT_PERMISSIONS => axum::http::StatusCode::FORBIDDEN,
                                ras_jsonrpc_types::error_codes::TOKEN_EXPIRED => axum::http::StatusCode::UNAUTHORIZED,
                                _ => axum::http::StatusCode::OK, // Other JSON-RPC errors still return 200 OK
                            }
                        } else {
                            axum::http::StatusCode::OK
                        };

                        (
                            status_code,
                            [("Content-Type", "application/json")],
                            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
                        )
                    }
                });

                let mut router = axum::Router::new();

                // Add the JSON-RPC endpoint
                router = router.route(&base_url, rpc_handler);

                // Include explorer routes if explorer is enabled
                #explorer_route_integration

                Ok(router)
            }

            async fn handle_request(&self, headers: axum::http::HeaderMap, body: String) -> ras_jsonrpc_types::JsonRpcResponse {
                // Parse JSON-RPC request
                let request: ras_jsonrpc_types::JsonRpcRequest = match serde_json::from_str(&body) {
                    Ok(req) => req,
                    Err(_) => return ras_jsonrpc_types::JsonRpcResponse::error(ras_jsonrpc_types::JsonRpcError::parse_error(), None),
                };

                let request_id = request.id.clone();

                // Validate JSON-RPC version
                if request.jsonrpc != "2.0" {
                    return ras_jsonrpc_types::JsonRpcResponse::error(ras_jsonrpc_types::JsonRpcError::invalid_request(), request_id);
                }

                // Try to authenticate user if auth provider is available
                let auth_result = if let Some(auth_provider) = &self.auth_provider {
                    if let Some(token) = headers
                        .get("Authorization")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.strip_prefix("Bearer ")) {
                        Some(auth_provider.authenticate(token.to_string()).await)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let authenticated_user = match auth_result {
                    Some(Ok(user)) => Some(user),
                    Some(Err(ras_jsonrpc_core::AuthError::TokenExpired)) => {
                        return ras_jsonrpc_types::JsonRpcResponse::error(
                            ras_jsonrpc_types::JsonRpcError::token_expired(),
                            request_id
                        );
                    }
                    _ => None,
                };

                // Call usage tracker if configured
                if let Some(tracker) = &self.usage_tracker {
                    let user_ref = authenticated_user.as_ref();
                    tracker(&headers, user_ref, &request).await;
                }

                // Dispatch method
                match request.method.as_str() {
                    #(#method_dispatch)*
                    _ => ras_jsonrpc_types::JsonRpcResponse::error(
                        ras_jsonrpc_types::JsonRpcError::method_not_found(&request.method),
                        request_id
                    )
                }
            }
        }
    }
}
