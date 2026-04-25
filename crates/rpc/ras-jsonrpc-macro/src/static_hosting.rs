use proc_macro2::TokenStream;
use quote::quote;

/// Configuration for static hosting of the JSON-RPC explorer
#[derive(Debug, Clone)]
pub struct StaticHostingConfig {
    /// Whether to serve the explorer
    pub serve_explorer: bool,
    /// Path where the explorer will be served
    pub explorer_path: String,
}

impl Default for StaticHostingConfig {
    fn default() -> Self {
        Self {
            serve_explorer: false,
            explorer_path: "/explorer".to_string(),
        }
    }
}

/// Generate code for static hosting of the JSON-RPC explorer
pub fn generate_static_hosting_code(
    config: &StaticHostingConfig,
    service_name: &syn::Ident,
    _base_path: &str,
) -> TokenStream {
    if !config.serve_explorer {
        return TokenStream::new();
    }

    const TEMPLATE_CONTENT: &str = include_str!("api_explorer_template.html");

    let explorer_path_suffix = &config.explorer_path;
    let service_name_str = service_name.to_string();
    let service_name_lower = service_name_str.to_lowercase();
    let openrpc_fn_name_str = ["generate_", &service_name_lower, "_openrpc"].concat();
    let explorer_routes_fn_str = [&service_name_lower, "_explorer_routes"].concat();
    let openrpc_fn_name = syn::Ident::new(&openrpc_fn_name_str, service_name.span());
    let explorer_routes_fn = syn::Ident::new(&explorer_routes_fn_str, service_name.span());

    // Embed the template as a string literal
    let template_lit = syn::LitStr::new(TEMPLATE_CONTENT, proc_macro2::Span::call_site());

    quote! {
        /// Routes for the JSON-RPC explorer
        pub fn #explorer_routes_fn(base_path: &str) -> ::axum::Router {
            use ::axum::{response::Html, routing::get, Json};

            let explorer_path = format!("{}{}", base_path, #explorer_path_suffix);
            let openrpc_path = format!("{}/openrpc.json", &explorer_path);

            let serve_explorer = {
                let base_path = base_path.to_string();
                let openrpc_path = openrpc_path.clone();
                move || {
                    let base_path = base_path.clone();
                    let openrpc_path = openrpc_path.clone();
                    async move {
                        const TEMPLATE: &str = #template_lit;

                        let html = TEMPLATE
                            .replace("{SERVICE_NAME_JSON}", &::serde_json::to_string(#service_name_str).unwrap_or_else(|_| "\"API\"".to_string()))
                            .replace("{PROTOCOL_JSON}", &::serde_json::to_string("jsonrpc").unwrap_or_else(|_| "\"jsonrpc\"".to_string()))
                            .replace("{SPEC_PATH_JSON}", &::serde_json::to_string(&openrpc_path).unwrap_or_else(|_| "\"openrpc.json\"".to_string()))
                            .replace("{API_BASE_PATH_JSON}", &::serde_json::to_string(&base_path).unwrap_or_else(|_| "\"/\"".to_string()));

                        Html(html)
                    }
                }
            };

            async fn serve_openrpc() -> Json<::serde_json::Value> {
                let doc = #openrpc_fn_name();
                Json(::serde_json::to_value(doc).unwrap())
            }

            ::axum::Router::new()
                .route(&explorer_path, get(serve_explorer))
                .route(&openrpc_path, get(serve_openrpc))
        }
    }
}
