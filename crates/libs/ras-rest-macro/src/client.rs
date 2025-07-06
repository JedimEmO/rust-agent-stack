use crate::{EndpointDefinition, HttpMethod, ServiceDefinition};
use quote::quote;

/// Generate client code for REST service
pub fn generate_client_code(service_def: &ServiceDefinition) -> proc_macro2::TokenStream {
    let service_name = &service_def.service_name;
    let client_name = quote::format_ident!("{}Client", service_name);
    let client_builder_name = quote::format_ident!("{}ClientBuilder", service_name);
    let base_path = &service_def.base_path;

    // Generate client methods
    let client_methods = service_def.endpoints.iter().map(generate_client_method);

    let client_methods_with_timeout = service_def
        .endpoints
        .iter()
        .map(generate_client_method_with_timeout);

    let output = quote! {
        #[cfg(feature = "client")]
        /// Helper function to join URL segments properly
        fn join_url_segments(base: &str, path: &str) -> String {
            let base = base.trim_end_matches('/');
            let path = path.trim_start_matches('/');
            if path.is_empty() {
                base.to_string()
            } else {
                format!("{}/{}", base, path)
            }
        }

        #[cfg(feature = "client")]
        /// Generated client for the REST service
        #[derive(Clone)]
        pub struct #client_name {
            client: reqwest::Client,
            server_url: String,
            base_path: String,
            bearer_token: Option<String>,
            default_timeout: Option<std::time::Duration>,
        }

        #[cfg(feature = "client")]
        /// Builder for the REST client
        pub struct #client_builder_name {
            server_url: String,
            timeout: Option<std::time::Duration>,
        }

        #[cfg(feature = "client")]
        impl #client_builder_name {
            /// Create a new client builder with the required server URL
            pub fn new(server_url: impl Into<String>) -> Self {
                Self {
                    server_url: server_url.into(),
                    timeout: None,
                }
            }

            /// Set the default timeout for requests
            pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
                self.timeout = Some(timeout);
                self
            }

            /// Build the client
            /// 
            /// # Errors
            /// 
            /// Returns an error if the underlying HTTP client fails to build
            pub fn build(self) -> Result<#client_name, Box<dyn std::error::Error + Send + Sync>> {
                let mut client_builder = reqwest::Client::builder();
                
                // Timeout is not supported in WASM builds
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(timeout) = self.timeout {
                    client_builder = client_builder.timeout(timeout);
                }

                let client = client_builder.build()?;

                Ok(#client_name {
                    client,
                    server_url: self.server_url,
                    base_path: #base_path.to_string(),
                    bearer_token: None,
                    default_timeout: self.timeout,
                })
            }
        }

        #[cfg(feature = "client")]
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
        }
    };

    output
}

/// Generate a client method for the REST service
fn generate_client_method(endpoint: &EndpointDefinition) -> proc_macro2::TokenStream {
    let method_name = &endpoint.handler_name;

    // Build function parameters and call arguments
    let mut params = Vec::new();
    let mut call_args = Vec::new();

    // Add path parameters
    for path_param in endpoint.path_params.iter() {
        let param_name = &path_param.name;
        let param_type = &path_param.param_type;
        params.push(quote! { #param_name: #param_type });
        call_args.push(quote! { #param_name });
    }

    // Add request body parameter if present
    if endpoint.request_type.is_some() {
        let request_type = endpoint.request_type.as_ref().unwrap();
        params.push(quote! { body: #request_type });
        call_args.push(quote! { body });
    }

    let response_type = &endpoint.response_type;
    let method_name_with_timeout = quote::format_ident!("{}_with_timeout", method_name);

    quote! {
        /// Call the #method_name endpoint
        pub async fn #method_name(&self, #(#params),*) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>> {
            self.#method_name_with_timeout(#(#call_args,)* None).await
        }
    }
}

/// Generate a client method with timeout for the REST service
fn generate_client_method_with_timeout(endpoint: &EndpointDefinition) -> proc_macro2::TokenStream {
    let method_name = &endpoint.handler_name;
    let method_name_with_timeout = quote::format_ident!("{}_with_timeout", method_name);
    let http_method = match endpoint.method {
        HttpMethod::Get => quote! { reqwest::Method::GET },
        HttpMethod::Post => quote! { reqwest::Method::POST },
        HttpMethod::Put => quote! { reqwest::Method::PUT },
        HttpMethod::Delete => quote! { reqwest::Method::DELETE },
        HttpMethod::Patch => quote! { reqwest::Method::PATCH },
    };

    let path = &endpoint.path;

    // Build function parameters
    let mut params = Vec::new();
    let mut path_substitutions = Vec::new();
    let mut param_names = Vec::new();
    // Build URL construction with proper joining
    let mut url_construction = quote! {
        join_url_segments(&join_url_segments(&self.server_url, &self.base_path), #path)
    };

    // Add path parameters
    for path_param in endpoint.path_params.iter() {
        let param_name = &path_param.name;
        let param_type = &path_param.param_type;
        params.push(quote! { #param_name: #param_type });
        param_names.push(param_name);

        // Handle path parameter substitution
        let placeholder = format!("{{{}}}", param_name);
        path_substitutions.push(quote! {
            .replace(#placeholder, &#param_name.to_string())
        });
    }

    // Update URL construction if there are path parameters
    if !path_substitutions.is_empty() {
        url_construction = quote! {
            join_url_segments(&join_url_segments(&self.server_url, &self.base_path), #path)
            #(#path_substitutions)*
        };
    }

    // Add request body parameter if present
    let request_body_handling = if let Some(request_type) = &endpoint.request_type {
        params.push(quote! { body: #request_type });
        quote! {
            request_builder = request_builder.json(&body);
        }
    } else {
        quote! {}
    };

    let response_type = &endpoint.response_type;
    
    // Check if response type is unit type ()
    let is_unit_type = quote!(#response_type).to_string() == "()";
    
    let response_handling = if is_unit_type {
        quote! {
            if response.status().is_success() {
                Ok(())
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("HTTP error {}: {}", status, error_text).into())
            }
        }
    } else {
        quote! {
            if response.status().is_success() {
                let result = response.json().await?;
                Ok(result)
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("HTTP error {}: {}", status, error_text).into())
            }
        }
    };

    quote! {
        /// Call the #method_name endpoint with a custom timeout
        pub async fn #method_name_with_timeout(
            &self,
            #(#params,)*
            timeout: Option<std::time::Duration>
        ) -> Result<#response_type, Box<dyn std::error::Error + Send + Sync>> {
            let url = #url_construction;

            let mut request_builder = self.client
                .request(#http_method, &url);

            // Add bearer token if available
            if let Some(token) = &self.bearer_token {
                request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
            }

            #request_body_handling

            // Override timeout if provided (not supported in WASM builds)
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(timeout) = timeout {
                request_builder = request_builder.timeout(timeout);
            }

            let response = request_builder.send().await?;

            #response_handling
        }
    }
}
