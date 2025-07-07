use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::parser::{Endpoint, FileServiceDefinition, Operation};

pub fn generate_client(definition: &FileServiceDefinition) -> TokenStream {
    let service_name = &definition.service_name;
    let base_path = &definition.base_path;

    let client_name = format_ident!("{}Client", service_name);
    let builder_name = format_ident!("{}ClientBuilder", service_name);

    let client_methods = generate_client_methods(&definition.endpoints, &base_path.value());

    quote! {
        pub struct #client_name {
            client: ::reqwest::Client,
            base_url: String,
            bearer_token: ::std::sync::RwLock<Option<String>>,
        }

        impl #client_name {
            pub fn builder(base_url: impl Into<String>) -> #builder_name {
                #builder_name::new(base_url)
            }

            pub fn set_bearer_token(&self, token: Option<impl Into<String>>) {
                *self.bearer_token.write().unwrap() = token.map(|t| t.into());
            }

            fn build_request(&self, method: ::reqwest::Method, path: &str) -> ::reqwest::RequestBuilder {
                let mut req = self.client.request(method, format!("{}{}", self.base_url, path));

                if let Some(token) = self.bearer_token.read().unwrap().as_ref() {
                    req = req.header("Authorization", format!("Bearer {}", token));
                }

                req
            }

            #client_methods
        }

        pub struct #builder_name {
            base_url: String,
            client: Option<::reqwest::Client>,
            #[cfg(not(target_arch = "wasm32"))]
            timeout: Option<std::time::Duration>,
        }

        impl #builder_name {
            pub fn new(base_url: impl Into<String>) -> Self {
                Self {
                    base_url: base_url.into(),
                    client: None,
                    #[cfg(not(target_arch = "wasm32"))]
                    timeout: None,
                }
            }

            pub fn with_client(mut self, client: ::reqwest::Client) -> Self {
                self.client = Some(client);
                self
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
                self.timeout = Some(timeout);
                self
            }

            pub fn build(self) -> Result<#client_name, Box<dyn std::error::Error>> {
                let client = match self.client {
                    Some(client) => client,
                    None => {
                        let mut builder = ::reqwest::Client::builder();

                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(timeout) = self.timeout {
                            builder = builder.timeout(timeout);
                        }

                        builder.build()?
                    }
                };

                Ok(#client_name {
                    client,
                    base_url: self.base_url,
                    bearer_token: ::std::sync::RwLock::new(None),
                })
            }
        }
    }
}

fn generate_client_methods(endpoints: &[Endpoint], base_path: &str) -> TokenStream {
    let methods = endpoints.iter().flat_map(|endpoint| {
        let method_name = &endpoint.name;
        let timeout_method_name = format_ident!("{}_with_timeout", method_name);

        let path = endpoint.path.as_ref()
            .map(|p| p.value())
            .unwrap_or_else(|| endpoint.name.to_string());
        let full_path = format!("{}/{}", base_path, path);

        let path_params: Vec<_> = endpoint.path_params.iter().map(|param| {
            let name = &param.name;
            let ty = &param.ty;
            quote! { #name: #ty }
        }).collect();

        let path_construction = if endpoint.path_params.is_empty() {
            quote! { #full_path }
        } else {
            let replacements = endpoint.path_params.iter().map(|param| {
                let name = &param.name;
                let placeholder = format!("{{{}}}", name);
                quote! { .replace(#placeholder, &#name.to_string()) }
            });
            quote! { #full_path.to_string()#(#replacements)* }
        };

        let path_arg_names: Vec<_> = endpoint.path_params.iter().map(|param| {
            &param.name
        }).collect();

        match &endpoint.operation {
            Operation::Upload => {
                let response_type = endpoint.response_type.as_ref()
                    .map(|t| quote! { #t })
                    .unwrap_or_else(|| quote! { () });

                let main_method = quote! {
                    pub async fn #method_name(
                        &self,
                        #(#path_params,)*
                        file_path: impl AsRef<std::path::Path>,
                        file_name: Option<&str>,
                        content_type: Option<&str>
                    ) -> Result<#response_type, Box<dyn std::error::Error>> {
                        let path = #path_construction;

                        let file = ::tokio::fs::File::open(file_path.as_ref()).await?;
                        let stream = ::tokio_util::io::ReaderStream::new(file);
                        let body = ::reqwest::Body::wrap_stream(stream);

                        let file_name = file_name.unwrap_or_else(|| {
                            file_path.as_ref()
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("file")
                        });

                        let part = ::reqwest::multipart::Part::stream(body)
                            .file_name(file_name.to_string());

                        let part = if let Some(ct) = content_type {
                            part.mime_str(ct)?
                        } else {
                            part
                        };

                        let form = ::reqwest::multipart::Form::new()
                            .part("file", part);

                        let response = self.build_request(::reqwest::Method::POST, &path)
                            .multipart(form)
                            .send()
                            .await?;

                        if !response.status().is_success() {
                            let status = response.status();
                            let text = response.text().await?;
                            return Err(format!("Upload failed with status {}: {}", status, text).into());
                        }

                        Ok(response.json().await?)
                    }
                };

                let timeout_method = quote! {
                    #[cfg(not(target_arch = "wasm32"))]
                    pub async fn #timeout_method_name(
                        &self,
                        #(#path_params,)*
                        file_path: impl AsRef<std::path::Path>,
                        file_name: Option<&str>,
                        content_type: Option<&str>,
                        timeout: std::time::Duration
                    ) -> Result<#response_type, Box<dyn std::error::Error>> {
                        ::tokio::time::timeout(
                            timeout,
                            self.#method_name(#(#path_arg_names,)* file_path, file_name, content_type)
                        ).await?
                    }
                };

                vec![main_method, timeout_method]
            }
            Operation::Download => {
                let main_method = quote! {
                    pub async fn #method_name(
                        &self,
                        #(#path_params,)*
                    ) -> Result<::reqwest::Response, Box<dyn std::error::Error>> {
                        let path = #path_construction;

                        let response = self.build_request(::reqwest::Method::GET, &path)
                            .send()
                            .await?;

                        if !response.status().is_success() {
                            let status = response.status();
                            let text = response.text().await?;
                            return Err(format!("Download failed with status {}: {}", status, text).into());
                        }

                        Ok(response)
                    }
                };

                let timeout_method = quote! {
                    #[cfg(not(target_arch = "wasm32"))]
                    pub async fn #timeout_method_name(
                        &self,
                        #(#path_params,)*
                        timeout: std::time::Duration
                    ) -> Result<::reqwest::Response, Box<dyn std::error::Error>> {
                        ::tokio::time::timeout(
                            timeout,
                            self.#method_name(#(#path_arg_names,)*)
                        ).await?
                    }
                };

                vec![main_method, timeout_method]
            }
        }
    });

    quote! { #(#methods)* }
}
