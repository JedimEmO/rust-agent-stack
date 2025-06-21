use crate::{MethodDefinition, ServiceDefinition};
use quote::quote;

/// Generate client code for JSON-RPC service
pub fn generate_client_code(service_def: &ServiceDefinition) -> proc_macro2::TokenStream {
    let service_name = &service_def.service_name;
    let client_name = quote::format_ident!("{}Client", service_name);
    let client_builder_name = quote::format_ident!("{}ClientBuilder", service_name);

    // Generate client methods
    let client_methods = service_def.methods.iter().map(generate_client_method);

    let client_methods_with_timeout = service_def
        .methods
        .iter()
        .map(generate_client_method_with_timeout);

    let output = quote! {
        /// Generated client for the JSON-RPC service
        #[derive(Clone)]
        pub struct #client_name {
            client: reqwest::Client,
            server_url: String,
            bearer_token: Option<String>,
            default_timeout: Option<std::time::Duration>,
        }

        /// Builder for the JSON-RPC client
        pub struct #client_builder_name {
            server_url: Option<String>,
            timeout: Option<std::time::Duration>,
        }

        impl #client_builder_name {
            /// Create a new client builder
            pub fn new() -> Self {
                Self {
                    server_url: None,
                    timeout: None,
                }
            }

            /// Set the server URL
            pub fn server_url(mut self, url: impl Into<String>) -> Self {
                self.server_url = Some(url.into());
                self
            }

            /// Set the default timeout for requests
            pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
                self.timeout = Some(timeout);
                self
            }

            /// Build the client
            pub fn build(self) -> Result<#client_name, Box<dyn std::error::Error + Send + Sync>> {
                let server_url = self.server_url.ok_or("Server URL is required")?;

                let mut client_builder = reqwest::Client::builder();
                
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(timeout) = self.timeout {
                    client_builder = client_builder.timeout(timeout);
                }

                let client = client_builder.build()?;

                Ok(#client_name {
                    client,
                    server_url,
                    bearer_token: None,
                    default_timeout: self.timeout,
                })
            }
        }

        impl #client_name {
            /// Set the bearer token for authentication
            pub fn set_bearer_token(&mut self, token: Option<impl Into<String>>) {
                self.bearer_token = token.map(|t| t.into());
            }

            /// Get a reference to the bearer token
            pub fn bearer_token(&self) -> Option<&str> {
                self.bearer_token.as_deref()
            }

            #(#client_methods)*
            #(#client_methods_with_timeout)*

            /// Make a JSON-RPC request with optional timeout
            async fn make_request<T, R>(
                &self,
                method: &str,
                params: T,
                timeout: Option<std::time::Duration>,
            ) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
            where
                T: serde::Serialize,
                R: serde::de::DeserializeOwned,
            {
                let request_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": method,
                    "params": params,
                    "id": 1
                });

                let mut request_builder = self.client
                    .post(&self.server_url)
                    .header("Content-Type", "application/json")
                    .json(&request_body);

                // Add bearer token if available
                if let Some(token) = &self.bearer_token {
                    request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
                }

                // Override timeout if provided (not supported in WASM)
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(timeout) = timeout {
                    request_builder = request_builder.timeout(timeout);
                }

                let response = request_builder.send().await?;
                let json_response: serde_json::Value = response.json().await?;

                // Check for JSON-RPC error
                if let Some(error) = json_response.get("error") {
                    return Err(format!("JSON-RPC error: {}", error).into());
                }

                // Extract result
                let result = json_response.get("result")
                    .ok_or("Missing result in JSON-RPC response")?;

                let deserialized_result: R = serde_json::from_value(result.clone())?;
                Ok(deserialized_result)
            }
        }
    };

    output
}

/// Generate a client method for the JSON-RPC service
fn generate_client_method(method: &MethodDefinition) -> proc_macro2::TokenStream {
    let method_name = &method.name;
    let method_str = method_name.to_string();
    let request_type = &method.request_type;
    let response_type = &method.response_type;

    quote! {
        /// Call the #method_name method
        pub async fn #method_name(&self, params: #request_type) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>> {
            self.make_request(#method_str, params, None).await
        }
    }
}

/// Generate a client method with timeout for the JSON-RPC service
fn generate_client_method_with_timeout(method: &MethodDefinition) -> proc_macro2::TokenStream {
    let method_name = &method.name;
    let method_name_with_timeout = quote::format_ident!("{}_with_timeout", method_name);
    let method_str = method_name.to_string();
    let request_type = &method.request_type;
    let response_type = &method.response_type;

    quote! {
        /// Call the #method_name method with a custom timeout
        pub async fn #method_name_with_timeout(
            &self,
            params: #request_type,
            timeout: std::time::Duration
        ) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>> {
            self.make_request(#method_str, params, Some(timeout)).await
        }
    }
}
