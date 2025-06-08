use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Token, Type, parse::Parse, parse_macro_input};

mod openrpc;

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
    methods: Vec<MethodDefinition>,
}

#[derive(Debug)]
enum OpenRpcConfig {
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
    WithPermissions(Vec<String>),
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
        if content.peek(Ident) {
            let field_name = content.fork().parse::<Ident>()?;
            if field_name == "openrpc" {
                let _ = content.parse::<Ident>()?; // "openrpc"
                let _ = content.parse::<Token![:]>()?;

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

                let _ = content.parse::<Token![,]>()?;
            }
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
                    // Parse ([...])
                    let perms_content;
                    syn::parenthesized!(perms_content in input);

                    let perms_array_content;
                    syn::bracketed!(perms_array_content in perms_content);

                    let mut permissions = Vec::new();
                    while !perms_array_content.is_empty() {
                        let perm = perms_array_content.parse::<LitStr>()?;
                        permissions.push(perm.value());

                        if perms_array_content.peek(Token![,]) {
                            let _ = perms_array_content.parse::<Token![,]>()?;
                        }
                    }

                    AuthRequirement::WithPermissions(permissions)
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
    let service_name = &service_def.service_name;
    let service_trait_name = quote::format_ident!("{}Trait", service_name);
    let builder_name = quote::format_ident!("{}Builder", service_name);

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
                    async fn #method_name(&self, user: &rust_jsonrpc_core::AuthenticatedUser, request: #request_type) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>>;
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
                    #field_name: Option<Box<dyn Fn(rust_jsonrpc_core::AuthenticatedUser, #request_type) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<#response_type, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>>,
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
                        F: Fn(rust_jsonrpc_core::AuthenticatedUser, #request_type) -> Fut + Send + Sync + 'static,
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

    // Generate method dispatch logic for the JSON-RPC handler
    let method_dispatch = service_def.methods.iter().map(|method| {
        let method_name = &method.name;
        let method_str = method_name.to_string();
        let field_name = quote::format_ident!("{}_handler", method_name);
        let request_type = &method.request_type;
        let permissions = match &method.auth {
            AuthRequirement::Unauthorized => Vec::new(),
            AuthRequirement::WithPermissions(perms) => perms.clone(),
        };

        match &method.auth {
            AuthRequirement::Unauthorized => {
                quote! {
                    #method_str => {
                        if let Some(handler) = &self.#field_name {
                            // Parse parameters
                            let params: #request_type = if let Some(params) = request.params {
                                serde_json::from_value(params)
                                    .map_err(|e| rust_jsonrpc_types::JsonRpcError::invalid_params(e.to_string()))?
                            } else {
                                return Err(rust_jsonrpc_types::JsonRpcError::invalid_params("Missing parameters".to_string()));
                            };

                            // Call handler
                            match handler(params).await {
                                Ok(result) => {
                                    let result_value = serde_json::to_value(result)
                                        .map_err(|e| rust_jsonrpc_types::JsonRpcError::internal_error(e.to_string()))?;
                                    Ok(rust_jsonrpc_types::JsonRpcResponse::success(result_value, request.id.clone()))
                                }
                                Err(e) => Err(rust_jsonrpc_types::JsonRpcError::internal_error(e.to_string())),
                            }
                        } else {
                            Err(rust_jsonrpc_types::JsonRpcError::method_not_found(&#method_str))
                        }
                    }
                }
            }
            AuthRequirement::WithPermissions(_) => {
                quote! {
                    #method_str => {
                        if let Some(handler) = &self.#field_name {
                            // Extract and validate auth token
                            let token = headers
                                .get("Authorization")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.strip_prefix("Bearer "))
                                .ok_or_else(|| rust_jsonrpc_types::JsonRpcError::authentication_required())?;

                            // Authenticate user
                            let user = self.auth_provider
                                .as_ref()
                                .ok_or_else(|| rust_jsonrpc_types::JsonRpcError::internal_error("No auth provider configured".to_string()))?
                                .authenticate(token.to_string())
                                .await
                                .map_err(|e| rust_jsonrpc_types::JsonRpcError::authentication_required())?;

                            // Check permissions
                            let required_permissions = vec![#(#permissions.to_string()),*];
                            if !required_permissions.is_empty() {
                                self.auth_provider
                                    .as_ref()
                                    .unwrap()
                                    .check_permissions(&user, &required_permissions)
                                    .map_err(|e| match e {
                                        rust_jsonrpc_core::AuthError::InsufficientPermissions { required, has } => {
                                            rust_jsonrpc_types::JsonRpcError::insufficient_permissions(required, has)
                                        }
                                        _ => rust_jsonrpc_types::JsonRpcError::authentication_required(),
                                    })?;
                            }

                            // Parse parameters
                            let params: #request_type = if let Some(params) = request.params {
                                serde_json::from_value(params)
                                    .map_err(|e| rust_jsonrpc_types::JsonRpcError::invalid_params(e.to_string()))?
                            } else {
                                return Err(rust_jsonrpc_types::JsonRpcError::invalid_params("Missing parameters".to_string()));
                            };

                            // Call handler
                            match handler(user, params).await {
                                Ok(result) => {
                                    let result_value = serde_json::to_value(result)
                                        .map_err(|e| rust_jsonrpc_types::JsonRpcError::internal_error(e.to_string()))?;
                                    Ok(rust_jsonrpc_types::JsonRpcResponse::success(result_value, request.id.clone()))
                                }
                                Err(e) => Err(rust_jsonrpc_types::JsonRpcError::internal_error(e.to_string())),
                            }
                        } else {
                            Err(rust_jsonrpc_types::JsonRpcError::method_not_found(&#method_str))
                        }
                    }
                }
            }
        }
    });

    let output = quote! {
        /// Generated service trait
        pub trait #service_trait_name {
            #(#trait_methods)*
        }

        /// Generated builder for the JSON-RPC service
        pub struct #builder_name {
            base_url: String,
            auth_provider: Option<Box<dyn rust_jsonrpc_core::AuthProvider>>,
            #(#builder_fields)*
        }

        #openrpc_code
        #schema_checks

        impl #builder_name {
            /// Create a new builder with the base URL
            pub fn new(base_url: impl Into<String>) -> Self {
                Self {
                    base_url: base_url.into(),
                    auth_provider: None,
                    #(#field_inits,)*
                }
            }

            /// Set the auth provider
            pub fn auth_provider<T: rust_jsonrpc_core::AuthProvider>(mut self, provider: T) -> Self {
                self.auth_provider = Some(Box::new(provider));
                self
            }

            #(#builder_setters)*

            /// Build the axum router for the JSON-RPC service
            pub fn build(self) -> axum::Router {
                let base_url = self.base_url.clone();
                let service = std::sync::Arc::new(self);

                axum::Router::new()
                    .route(&base_url, axum::routing::post(move |headers: axum::http::HeaderMap, body: String| {
                        let service = service.clone();
                        async move {
                            match service.handle_request(headers, body).await {
                                Ok(response) => {
                                    (
                                        axum::http::StatusCode::OK,
                                        [("Content-Type", "application/json")],
                                        serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
                                    )
                                }
                                Err(error) => {
                                    let error_response = rust_jsonrpc_types::JsonRpcResponse::error(error, None);
                                    (
                                        axum::http::StatusCode::OK,  // JSON-RPC errors still return 200 OK
                                        [("Content-Type", "application/json")],
                                        serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string())
                                    )
                                }
                            }
                        }
                    }))
            }

            async fn handle_request(&self, headers: axum::http::HeaderMap, body: String) -> Result<rust_jsonrpc_types::JsonRpcResponse, rust_jsonrpc_types::JsonRpcError> {
                // Parse JSON-RPC request
                let request: rust_jsonrpc_types::JsonRpcRequest = serde_json::from_str(&body)
                    .map_err(|_| rust_jsonrpc_types::JsonRpcError::parse_error())?;

                // Validate JSON-RPC version
                if request.jsonrpc != "2.0" {
                    return Err(rust_jsonrpc_types::JsonRpcError::invalid_request());
                }

                // Dispatch method
                match request.method.as_str() {
                    #(#method_dispatch)*
                    _ => Err(rust_jsonrpc_types::JsonRpcError::method_not_found(&request.method))
                }
            }
        }
    };

    Ok(output)
}
