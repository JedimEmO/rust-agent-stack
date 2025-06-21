use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Token, Type, parse::Parse, parse_macro_input};

mod client;
mod openapi;
mod static_hosting;

/// Macro to generate a REST service with authentication support
///
/// This macro generates a service trait and builder that integrates with axum
/// for handling REST requests with authentication and authorization.
///
/// Supports HTTP methods: GET, POST, PUT, DELETE, PATCH
/// Supports path parameters and request bodies
/// Generates OpenAPI 3.0 documents using schemars
///
/// # Example
///
/// ```ignore
/// use ras_rest_macro::rest_service;
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
/// use axum::response::IntoResponse;
///
/// #[derive(Serialize, Deserialize, JsonSchema)]
/// struct UsersResponse {
///     users: Vec<()>,
/// }
///
/// #[derive(Serialize, Deserialize, JsonSchema)]
/// struct CreateUserRequest {
///     name: String,
/// }
///
/// #[derive(Serialize, Deserialize, JsonSchema)]
/// struct UserResponse {
///     id: String,
///     name: String,
/// }
///
/// #[derive(Serialize, Deserialize, JsonSchema)]
/// struct UpdateUserRequest {
///     name: String,
/// }
///
/// rest_service!({
///     service_name: UserService,
///     base_path: "/api/v1",
///     openapi: true,
///     serve_docs: true,
///     docs_path: "/docs",
///     ui_theme: "default",
///     endpoints: [
///         GET UNAUTHORIZED users() -> UsersResponse,
///         POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> UserResponse,
///         GET WITH_PERMISSIONS(["user"]) users/{id: String}() -> UserResponse,
///         PUT WITH_PERMISSIONS(["admin"]) users/{id: String}(UpdateUserRequest) -> UserResponse,
///         DELETE WITH_PERMISSIONS(["admin"]) users/{id: String}() -> (),
///     ]
/// });
/// ```
#[proc_macro]
pub fn rest_service(input: TokenStream) -> TokenStream {
    let service_definition = parse_macro_input!(input as ServiceDefinition);

    match generate_service_code(service_definition) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[derive(Debug)]
struct ServiceDefinition {
    service_name: Ident,
    base_path: String,
    openapi: Option<OpenApiConfig>,
    static_hosting: static_hosting::StaticHostingConfig,
    endpoints: Vec<EndpointDefinition>,
}

#[derive(Debug)]
enum OpenApiConfig {
    Enabled,
    WithPath(String),
}

#[derive(Debug)]
struct EndpointDefinition {
    method: HttpMethod,
    auth: AuthRequirement,
    path: String,
    path_params: Vec<PathParam>,
    request_type: Option<Type>,
    response_type: Type,
    handler_name: Ident,
}

#[derive(Debug)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl HttpMethod {
    fn as_axum_method(&self) -> proc_macro2::TokenStream {
        match self {
            HttpMethod::Get => quote! { axum::routing::get },
            HttpMethod::Post => quote! { axum::routing::post },
            HttpMethod::Put => quote! { axum::routing::put },
            HttpMethod::Delete => quote! { axum::routing::delete },
            HttpMethod::Patch => quote! { axum::routing::patch },
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
        }
    }
}

#[derive(Debug)]
struct PathParam {
    name: Ident,
    param_type: Type,
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

        // Parse base_path: "string"
        let _ = content.parse::<Ident>()?; // "base_path"
        let _ = content.parse::<Token![:]>()?;
        let base_path_lit = content.parse::<LitStr>()?;
        let base_path = base_path_lit.value();
        let _ = content.parse::<Token![,]>()?;

        // Parse optional fields (openapi, serve_docs, docs_path, ui_theme)
        let mut openapi = None;
        let mut static_hosting = static_hosting::StaticHostingConfig::default();

        // Parse optional fields
        while content.peek(Ident) {
            let field_name = content.fork().parse::<Ident>()?;

            if field_name == "openapi" {
                let _ = content.parse::<Ident>()?; // "openapi"
                let _ = content.parse::<Token![:]>()?;

                // Parse openapi value - can be true/false or { output: "path" }
                if content.peek(syn::LitBool) {
                    let enabled = content.parse::<syn::LitBool>()?;
                    if enabled.value() {
                        openapi = Some(OpenApiConfig::Enabled);
                    }
                } else if content.peek(syn::token::Brace) {
                    let openapi_content;
                    syn::braced!(openapi_content in content);

                    // Parse output: "path"
                    let _ = openapi_content.parse::<Ident>()?; // "output"
                    let _ = openapi_content.parse::<Token![:]>()?;
                    let path = openapi_content.parse::<LitStr>()?;
                    openapi = Some(OpenApiConfig::WithPath(path.value()));
                }

                let _ = content.parse::<Token![,]>()?;
            } else if field_name == "serve_docs" {
                let _ = content.parse::<Ident>()?; // "serve_docs"
                let _ = content.parse::<Token![:]>()?;
                let enabled = content.parse::<syn::LitBool>()?;
                static_hosting.serve_docs = enabled.value();
                let _ = content.parse::<Token![,]>()?;
            } else if field_name == "docs_path" {
                let _ = content.parse::<Ident>()?; // "docs_path"
                let _ = content.parse::<Token![:]>()?;
                let path = content.parse::<LitStr>()?;
                static_hosting.docs_path = path.value();
                let _ = content.parse::<Token![,]>()?;
            } else if field_name == "ui_theme" {
                let _ = content.parse::<Ident>()?; // "ui_theme"
                let _ = content.parse::<Token![:]>()?;
                let theme = content.parse::<LitStr>()?;
                static_hosting.ui_theme = theme.value();
                let _ = content.parse::<Token![,]>()?;
            } else if field_name == "endpoints" {
                break; // Start parsing endpoints
            } else {
                return Err(syn::Error::new(
                    field_name.span(),
                    format!("Unknown field: {}", field_name),
                ));
            }
        }

        // Parse endpoints: [...]
        let _ = content.parse::<Ident>()?; // "endpoints"
        let _ = content.parse::<Token![:]>()?;

        let endpoints_content;
        syn::bracketed!(endpoints_content in content);

        let mut endpoints = Vec::new();
        while !endpoints_content.is_empty() {
            let endpoint = endpoints_content.parse::<EndpointDefinition>()?;
            endpoints.push(endpoint);

            // Handle optional trailing comma
            if endpoints_content.peek(Token![,]) {
                let _ = endpoints_content.parse::<Token![,]>()?;
            }
        }

        Ok(ServiceDefinition {
            service_name,
            base_path,
            openapi,
            static_hosting,
            endpoints,
        })
    }
}

impl Parse for EndpointDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse HTTP method (GET, POST, PUT, DELETE, PATCH)
        let method_ident = input.parse::<Ident>()?;
        let method = match method_ident.to_string().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            _ => {
                return Err(syn::Error::new(
                    method_ident.span(),
                    "Expected GET, POST, PUT, DELETE, or PATCH",
                ));
            }
        };

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

        // Parse path with potential path parameters (e.g., users/{id: String}/posts/{post_id: i32})
        let mut path_segments = Vec::new();
        let mut path_params = Vec::new();
        let mut handler_name_parts = Vec::new();

        // First segment is always the base path segment
        let first_segment = input.parse::<Ident>()?;
        path_segments.push(first_segment.to_string());
        handler_name_parts.push(first_segment.to_string());

        // Parse additional path segments
        while input.peek(Token![/]) {
            let _ = input.parse::<Token![/]>()?;

            if input.peek(syn::token::Brace) {
                // Parse path parameter {name: Type}
                let param_content;
                syn::braced!(param_content in input);

                let param_name = param_content.parse::<Ident>()?;
                let _ = param_content.parse::<Token![:]>()?;
                let param_type = param_content.parse::<Type>()?;

                path_segments.push(format!("{{{}}}", param_name));
                path_params.push(PathParam {
                    name: param_name.clone(),
                    param_type,
                });
                handler_name_parts.push(format!("by_{}", param_name));
            } else {
                // Regular path segment
                let segment = input.parse::<Ident>()?;
                path_segments.push(segment.to_string());
                handler_name_parts.push(segment.to_string());
            }
        }

        let path = format!("/{}", path_segments.join("/"));

        // Generate handler name based on method and path
        let method_str = method.as_str().to_lowercase();
        let path_str = handler_name_parts.join("_");
        let handler_name = syn::parse_str::<Ident>(&format!("{}_{}", method_str, path_str))?;

        // Parse (RequestType) - optional for GET/DELETE
        let request_type = if input.peek(syn::token::Paren) {
            let request_content;
            syn::parenthesized!(request_content in input);
            if !request_content.is_empty() {
                Some(request_content.parse::<Type>()?)
            } else {
                None
            }
        } else {
            None
        };

        // Parse -> ResponseType
        let _ = input.parse::<Token![->]>()?;
        let response_type = input.parse::<Type>()?;

        Ok(EndpointDefinition {
            method,
            auth,
            path,
            path_params,
            request_type,
            response_type,
            handler_name,
        })
    }
}

fn generate_service_code(service_def: ServiceDefinition) -> syn::Result<proc_macro2::TokenStream> {
    let service_name = &service_def.service_name;
    let service_trait_name = quote::format_ident!("{}Trait", service_name);
    let builder_name = quote::format_ident!("{}Builder", service_name);
    let base_path = &service_def.base_path;

    // Generate OpenAPI code if enabled in the macro input
    let (openapi_code, schema_checks) = if service_def.openapi.is_some() {
        let openapi_config = service_def.openapi.as_ref().unwrap();
        (
            openapi::generate_openapi_code(&service_def, openapi_config),
            openapi::generate_schema_impl_checks(&service_def),
        )
    } else {
        (quote! {}, quote! {})
    };

    // Generate static hosting code if enabled
    let static_hosting_code =
        static_hosting::generate_static_hosting_code(&service_def, &service_def.static_hosting);

    // Generate client code
    let client_code = crate::client::generate_client_code(&service_def);

    // Generate trait methods
    let trait_methods = service_def.endpoints.iter().map(|endpoint| {
        let handler_name = &endpoint.handler_name;
        let response_type = &endpoint.response_type;

        // Build parameter list based on auth requirements and path params
        let mut params = Vec::new();
        // Add authenticated user parameter if needed
        match &endpoint.auth {
            AuthRequirement::Unauthorized => {}
            AuthRequirement::WithPermissions(_) => {
                params.push(quote! { user: &ras_auth_core::AuthenticatedUser });
            }
        }

        // Add path parameters
        for path_param in &endpoint.path_params {
            let param_name = &path_param.name;
            let param_type = &path_param.param_type;
            params.push(quote! { #param_name: #param_type });
        }

        // Add request body parameter if present
        if let Some(request_type) = &endpoint.request_type {
            params.push(quote! { request: #request_type });
        }

        quote! {
            async fn #handler_name(&self, #(#params),*) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>>;
        }
    });

    // Generate builder fields - similar pattern to JSON-RPC macro
    let builder_fields = service_def.endpoints.iter().map(|endpoint| {
        let handler_name = &endpoint.handler_name;
        let field_name = quote::format_ident!("{}_handler", handler_name);
        let response_type = &endpoint.response_type;

        // Build parameter list for handler closure
        let mut handler_params = Vec::new();

        // Add authenticated user parameter if needed
        match &endpoint.auth {
            AuthRequirement::Unauthorized => {}
            AuthRequirement::WithPermissions(_) => {
                handler_params.push(quote! { &ras_auth_core::AuthenticatedUser });
            }
        }

        // Add path parameters
        for path_param in &endpoint.path_params {
            let param_type = &path_param.param_type;
            handler_params.push(quote! { #param_type });
        }

        // Add request body parameter if present
        if let Some(request_type) = &endpoint.request_type {
            handler_params.push(quote! { #request_type });
        }

        quote! {
            #field_name: Option<std::sync::Arc<dyn Fn(#(#handler_params),*) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<#response_type, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>>,
        }
    });

    // Generate builder setters
    let builder_setters = service_def.endpoints.iter().map(|endpoint| {
        let handler_name = &endpoint.handler_name;
        let setter_name = quote::format_ident!("{}_handler", handler_name);
        let field_name = quote::format_ident!("{}_handler", handler_name);
        let response_type = &endpoint.response_type;

        // Build parameter list for handler closure
        let mut handler_params = Vec::new();
        let mut handler_args = Vec::new();

        // Add authenticated user parameter if needed
        match &endpoint.auth {
            AuthRequirement::Unauthorized => {}
            AuthRequirement::WithPermissions(_) => {
                handler_params.push(quote! { &ras_auth_core::AuthenticatedUser });
                handler_args.push(quote! { user });
            }
        }

        // Add path parameters
        for path_param in &endpoint.path_params {
            let param_name = &path_param.name;
            let param_type = &path_param.param_type;
            handler_params.push(quote! { #param_type });
            handler_args.push(quote! { #param_name });
        }

        // Add request body parameter if present
        if let Some(request_type) = &endpoint.request_type {
            handler_params.push(quote! { #request_type });
            handler_args.push(quote! { body });
        }

        quote! {
            pub fn #setter_name<F, Fut>(mut self, handler: F) -> Self
            where
                F: Fn(#(#handler_params),*) -> Fut + Send + Sync + 'static,
                Fut: std::future::Future<Output = Result<#response_type, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
            {
                self.#field_name = Some(std::sync::Arc::new(move |#(#handler_args),*| Box::pin(handler(#(#handler_args),*))));
                self
            }
        }
    });

    // Generate field initializations for the constructor
    let field_inits = service_def.endpoints.iter().map(|endpoint| {
        let field_name = quote::format_ident!("{}_handler", endpoint.handler_name);
        quote! { #field_name: None }
    });

    // Generate route registration
    let route_registrations = service_def.endpoints.iter().map(|endpoint| {
        let method_routing = endpoint.method.as_axum_method();
        let path = &endpoint.path;
        let field_name = quote::format_ident!("{}_handler", endpoint.handler_name);
        let permission_groups = match &endpoint.auth {
            AuthRequirement::Unauthorized => Vec::new(),
            AuthRequirement::WithPermissions(groups) => groups.clone(),
        };
        let method_str = endpoint.method.as_str();

        // Generate the axum handler based on endpoint configuration
        let axum_handler = generate_axum_handler(endpoint);
        let handler_body = generate_handler_body(endpoint, method_str, path);

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
            if let Some(handler) = &self.#field_name {
                let handler = handler.clone();
                let auth_provider = self.auth_provider.clone();
                let required_permission_groups: Vec<Vec<String>> = #permission_groups_code;
                let with_usage_tracker = self.with_usage_tracker.clone();
                let with_method_duration_tracker = self.with_method_duration_tracker.clone();

                router = router.route(#path, #method_routing({
                    move |#axum_handler| {
                        let handler = handler.clone();
                        let auth_provider = auth_provider.clone();
                        let required_permission_groups: Vec<Vec<String>> = required_permission_groups.clone();
                        let with_usage_tracker = with_usage_tracker.clone();
                        let with_method_duration_tracker = with_method_duration_tracker.clone();

                        async move {
                            #handler_body
                        }
                    }
                }));
            }
        }
    });

    // Generate static hosting route registration
    let static_routes =
        static_hosting::generate_static_routes(&service_def, &service_def.static_hosting);

    let output = quote! {
        #[cfg(feature = "server")]
        /// Generated service trait
        pub trait #service_trait_name {
            #(#trait_methods)*
        }

        #[cfg(feature = "server")]
        /// Generated builder for the REST service
        pub struct #builder_name {
            auth_provider: Option<std::sync::Arc<dyn ras_auth_core::AuthProvider>>,
            with_usage_tracker: Option<std::sync::Arc<dyn Fn(&axum::http::HeaderMap, Option<&ras_auth_core::AuthenticatedUser>, &str, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>>,
            with_method_duration_tracker: Option<std::sync::Arc<dyn Fn(&str, &str, Option<&ras_auth_core::AuthenticatedUser>, std::time::Duration) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>>,
            #(#builder_fields)*
        }

        #openapi_code
        #schema_checks
        #static_hosting_code

        #[cfg(feature = "server")]
        impl #builder_name {
            /// Create a new builder
            pub fn new() -> Self {
                Self {
                    auth_provider: None,
                    with_usage_tracker: None,
                    with_method_duration_tracker: None,
                    #(#field_inits,)*
                }
            }

            /// Set the auth provider
            pub fn auth_provider<T: ras_auth_core::AuthProvider>(mut self, provider: T) -> Self {
                self.auth_provider = Some(std::sync::Arc::new(provider));
                self
            }

            #(#builder_setters)*

            /// Set the usage tracker - called before each request
            /// The tracker receives the headers, authenticated user (if any), HTTP method, and path
            pub fn with_usage_tracker<F, Fut>(mut self, tracker: F) -> Self
            where
                F: Fn(&axum::http::HeaderMap, Option<&ras_auth_core::AuthenticatedUser>, &str, &str) -> Fut + Send + Sync + 'static,
                Fut: std::future::Future<Output = ()> + Send + 'static,
            {
                self.with_usage_tracker = Some(std::sync::Arc::new(move |headers, user, method, path| {
                    Box::pin(tracker(headers, user, method, path))
                }));
                self
            }

            /// Set the method duration tracker - called after each request completes
            /// The tracker receives the HTTP method, path, authenticated user (if any), and execution duration
            pub fn with_method_duration_tracker<F, Fut>(mut self, tracker: F) -> Self
            where
                F: Fn(&str, &str, Option<&ras_auth_core::AuthenticatedUser>, std::time::Duration) -> Fut + Send + Sync + 'static,
                Fut: std::future::Future<Output = ()> + Send + 'static,
            {
                self.with_method_duration_tracker = Some(std::sync::Arc::new(move |method, path, user, duration| {
                    Box::pin(tracker(method, path, user, duration))
                }));
                self
            }

            /// Build the axum router for the REST service
            pub fn build(self) -> axum::Router {
                let mut router = axum::Router::new();

                #(#route_registrations)*

                // Add static hosting routes if enabled
                #static_routes

                // Handle empty or root base path
                if #base_path.is_empty() || #base_path == "/" {
                    router
                } else {
                    axum::Router::new().nest(#base_path, router)
                }
            }
        }

        #client_code
    };

    Ok(output)
}

fn generate_axum_handler(endpoint: &EndpointDefinition) -> proc_macro2::TokenStream {
    let mut extractors = Vec::new();

    // Always add headers extraction for tracking purposes
    extractors.push(quote! { headers: axum::http::HeaderMap });

    // Add path parameter extractors
    if !endpoint.path_params.is_empty() {
        let path_param_types = endpoint.path_params.iter().map(|param| &param.param_type);
        if endpoint.path_params.len() == 1 {
            extractors.push(quote! { axum::extract::Path(path_params): axum::extract::Path<#(#path_param_types)*> });
        } else {
            extractors.push(quote! { axum::extract::Path(path_params): axum::extract::Path<(#(#path_param_types),*)> });
        }
    }

    // Add request body extractor if present - use Result to handle JSON parsing errors
    if endpoint.request_type.is_some() {
        extractors.push(quote! { body_result: Result<axum::extract::Json<_>, axum::extract::rejection::JsonRejection> });
    }

    quote! {
        #(#extractors),*
    }
}

fn generate_handler_body(
    endpoint: &EndpointDefinition,
    method: &str,
    path: &str,
) -> proc_macro2::TokenStream {
    // Handle authentication if required
    match &endpoint.auth {
        AuthRequirement::Unauthorized => {
            // Build argument list for unauthorized endpoint
            let mut args = Vec::new();

            // Add path parameters
            if endpoint.path_params.len() == 1 {
                args.push(quote! { path_params });
            } else {
                for (i, _) in endpoint.path_params.iter().enumerate() {
                    let idx = syn::Index::from(i);
                    args.push(quote! { path_params.#idx });
                }
            }

            // Handle JSON body extraction with error handling
            let json_handling = if endpoint.request_type.is_some() {
                args.push(quote! { body });
                quote! {
                    // Handle JSON parsing errors
                    let body = match body_result {
                        Ok(json) => json.0,
                        Err(_) => {
                            use axum::response::IntoResponse;
                            return (
                                axum::http::StatusCode::BAD_REQUEST,
                                axum::Json(serde_json::json!({
                                    "error": "Invalid JSON"
                                }))
                            ).into_response();
                        },
                    };
                }
            } else {
                quote! {}
            };

            quote! {
                #json_handling

                // Call usage tracker if configured (for unauthorized endpoints, headers come from handler params)
                if let Some(tracker) = &with_usage_tracker {
                    tracker(&headers, None, #method, #path).await;
                }

                // Track duration
                let start_time = std::time::Instant::now();

                let result = match handler(#(#args),*).await {
                    Ok(result) => {
                        use axum::response::IntoResponse;
                        (
                            axum::http::StatusCode::OK,
                            axum::Json(result)
                        ).into_response()
                    },
                    Err(e) => {
                        use axum::response::IntoResponse;
                        tracing::error!(error = ?e, "Internal server error");
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(serde_json::json!({
                                "error": "Internal server error"
                            }))
                        ).into_response()
                    },
                };

                // Call duration tracker if configured
                let duration = start_time.elapsed();
                if let Some(tracker) = &with_method_duration_tracker {
                    tracker(#method, #path, None, duration).await;
                }

                result
            }
        }
        AuthRequirement::WithPermissions(_) => {
            // Build argument list for authenticated endpoint
            let mut args = vec![quote! { &user }];

            // Add path parameters
            if endpoint.path_params.len() == 1 {
                args.push(quote! { path_params });
            } else {
                for (i, _) in endpoint.path_params.iter().enumerate() {
                    let idx = syn::Index::from(i);
                    args.push(quote! { path_params.#idx });
                }
            }

            // Handle JSON body extraction with error handling
            let json_handling = if endpoint.request_type.is_some() {
                args.push(quote! { body });
                quote! {
                    // Handle JSON parsing errors
                    let body = match body_result {
                        Ok(json) => json.0,
                        Err(_) => {
                            use axum::response::IntoResponse;
                            return (
                                axum::http::StatusCode::BAD_REQUEST,
                                axum::Json(serde_json::json!({
                                    "error": "Invalid JSON"
                                }))
                            ).into_response();
                        },
                    };
                }
            } else {
                quote! {}
            };

            quote! {
                #json_handling

                // Extract and validate auth token
                let token = match headers
                    .get("Authorization")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.strip_prefix("Bearer "))
                {
                    Some(token) => token,
                    None => {
                        use axum::response::IntoResponse;
                        return (
                            axum::http::StatusCode::UNAUTHORIZED,
                            axum::Json(serde_json::json!({
                                "error": "Missing or invalid Authorization header"
                            }))
                        ).into_response();
                    },
                };

                // Authenticate user
                let user = match &auth_provider {
                    Some(provider) => match provider.authenticate(token.to_string()).await {
                        Ok(user) => user,
                        Err(_) => {
                            use axum::response::IntoResponse;
                            return (
                                axum::http::StatusCode::UNAUTHORIZED,
                                axum::Json(serde_json::json!({
                                    "error": "Authentication failed"
                                }))
                            ).into_response();
                        },
                    },
                    None => {
                        use axum::response::IntoResponse;
                        return (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(serde_json::json!({
                                "error": "No auth provider configured"
                            }))
                        ).into_response();
                    },
                };

                // Check permissions - AND within groups, OR between groups
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
                            let group_result = auth_provider.as_ref().unwrap().check_permissions(&user, permission_group);
                            if group_result.is_ok() {
                                has_permission = true;
                                break;
                            }
                        }
                    }

                    if !has_permission {
                        use axum::response::IntoResponse;
                        return (
                            axum::http::StatusCode::FORBIDDEN,
                            axum::Json(serde_json::json!({
                                "error": "Insufficient permissions"
                            }))
                        ).into_response();
                    }
                }

                // Call usage tracker if configured
                if let Some(tracker) = &with_usage_tracker {
                    tracker(&headers, Some(&user), #method, #path).await;
                }

                // Track duration
                let start_time = std::time::Instant::now();

                let result = match handler(#(#args),*).await {
                    Ok(result) => {
                        use axum::response::IntoResponse;
                        (
                            axum::http::StatusCode::OK,
                            axum::Json(result)
                        ).into_response()
                    },
                    Err(e) => {
                        use axum::response::IntoResponse;
                        tracing::error!(error = ?e, "Internal server error");
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(serde_json::json!({
                                "error": "Internal server error"
                            }))
                        ).into_response()
                    },
                };

                // Call duration tracker if configured
                let duration = start_time.elapsed();
                if let Some(tracker) = &with_method_duration_tracker {
                    tracker(#method, #path, Some(&user), duration).await;
                }

                result
            }
        }
    }
}
