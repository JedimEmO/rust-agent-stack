//! OpenAPI 3.0 document generation module
//!
//! This module provides functionality to generate OpenAPI 3.0 specification documents
//! from the rest_service macro definitions.

use crate::{AuthRequirement, OpenApiConfig, ServiceDefinition};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

/// Generates OpenAPI document creation code
pub fn generate_openapi_code(
    service_def: &ServiceDefinition,
    config: &OpenApiConfig,
) -> TokenStream {
    let service_name = &service_def.service_name;
    let openapi_fn_name = quote::format_ident!(
        "generate_{}_openapi",
        service_name.to_string().to_lowercase()
    );
    let openapi_to_file_fn_name = quote::format_ident!(
        "generate_{}_openapi_to_file",
        service_name.to_string().to_lowercase()
    );
    let endpoint_info_struct_name = quote::format_ident!("{}OpenApiEndpointInfo", service_name);

    // Generate the output path based on config
    let output_path_code = match config {
        OpenApiConfig::Enabled => {
            let service_name_lower = service_name.to_string().to_lowercase();
            quote! {
                format!("target/openapi/{}.json", #service_name_lower)
            }
        }
        OpenApiConfig::WithPath(path) => {
            quote! {
                #path.to_string()
            }
        }
    };

    // Collect unique types for schema generation
    let mut unique_types = std::collections::HashMap::new();
    for endpoint in &service_def.endpoints {
        if let Some(request_type) = &endpoint.request_type {
            let request_type_str = quote!(#request_type).to_string();
            unique_types.insert(request_type_str, quote!(#request_type));
        }

        let response_type = &endpoint.response_type;
        let response_type_str = quote!(#response_type).to_string();
        unique_types.insert(response_type_str, quote!(#response_type));

        // Add path parameter types
        for path_param in &endpoint.path_params {
            let param_type = &path_param.param_type;
            let param_type_str = quote!(#param_type).to_string();
            unique_types.insert(param_type_str, quote!(#param_type));
        }
    }

    // Generate schema generation functions
    let schema_fns: Vec<TokenStream> = unique_types
        .iter()
        .map(|(type_name, type_tokens)| {
            if type_name == "()" {
                quote! {} // Skip unit type, we'll handle it separately
            } else {
                let fn_name = quote::format_ident!(
                    "_generate_schema_for_{}_{}",
                    service_name.to_string().to_lowercase(),
                    type_name
                        .replace("::", "_")
                        .replace("<", "_")
                        .replace(">", "_")
                        .replace(" ", "_")
                );
                quote! {
                    fn #fn_name() -> serde_json::Value {
                        let schema = schemars::schema_for!(#type_tokens);
                        serde_json::to_value(&schema).unwrap_or_else(|_| {
                            serde_json::json!({
                                "type": "object",
                                "description": format!("Schema for {}", #type_name)
                            })
                        })
                    }
                }
            }
        })
        .collect();

    // Generate schema collection code
    let schema_insertions: Vec<TokenStream> = unique_types
        .keys()
        .map(|type_name| {
            if type_name == "()" {
                quote! {
                    schemas.insert("()".to_string(), serde_json::json!({
                        "type": "null",
                        "description": "Unit type (empty response)"
                    }));
                }
            } else {
                let fn_name = quote::format_ident!(
                    "_generate_schema_for_{}_{}",
                    service_name.to_string().to_lowercase(),
                    type_name
                        .replace("::", "_")
                        .replace("<", "_")
                        .replace(">", "_")
                        .replace(" ", "_")
                );
                quote! {
                    schemas.insert(#type_name.to_string(), #fn_name());
                }
            }
        })
        .collect();

    // Generate endpoint info structs
    let endpoint_infos: Vec<TokenStream> = service_def
        .endpoints
        .iter()
        .map(|endpoint| {
            let method = endpoint.method.as_str();
            let path = &endpoint.path;
            let auth_required = matches!(endpoint.auth, AuthRequirement::WithPermissions(_));
            let permissions = match &endpoint.auth {
                AuthRequirement::Unauthorized => vec![],
                AuthRequirement::WithPermissions(perms) => perms.clone(),
            };

            let request_type_name = if let Some(request_type) = &endpoint.request_type {
                quote!(#request_type).to_string()
            } else {
                "()".to_string()
            };

            let response_type = &endpoint.response_type;
            let path_param_infos: Vec<TokenStream> = endpoint
                .path_params
                .iter()
                .map(|param| {
                    let param_name = param.name.to_string();
                    let param_type = &param.param_type;
                    let param_type_str = quote!(#param_type).to_string();
                    quote! {
                        (#param_name.to_string(), #param_type_str.to_string())
                    }
                })
                .collect();

            quote! {
                #endpoint_info_struct_name {
                    method: #method.to_string(),
                    path: #path.to_string(),
                    auth_required: #auth_required,
                    permissions: vec![#(#permissions.to_string()),*] as Vec<String>,
                    request_type_name: #request_type_name.to_string(),
                    response_type_name: stringify!(#response_type).to_string(),
                    path_params: vec![#(#path_param_infos),*] as Vec<(String, String)>,
                }
            }
        })
        .collect();

    quote! {
        #[derive(serde::Serialize)]
        struct #endpoint_info_struct_name {
            method: String,
            path: String,
            auth_required: bool,
            permissions: Vec<String>,
            request_type_name: String,
            response_type_name: String,
            path_params: Vec<(String, String)>, // (name, type)
        }

        // Generate schema functions for each type
        #(#schema_fns)*

        /// Generate OpenAPI 3.0 document for this service
        pub fn #openapi_fn_name() -> serde_json::Value {
            use serde_json::json;
            use schemars::{schema_for, JsonSchema};
            use std::collections::HashMap;

            let endpoints: Vec<#endpoint_info_struct_name> = vec![
                #(#endpoint_infos),*
            ];

            // Generate schemas for all unique types
            let mut schemas = HashMap::new();

            // Insert all the generated schemas
            #(#schema_insertions)*

            // Group endpoints by path to create OpenAPI paths
            let mut paths = serde_json::Map::new();

            for endpoint in &endpoints {
                let path_item = paths.entry(endpoint.path.clone()).or_insert_with(|| json!({}));

                let method_lower = endpoint.method.to_lowercase();
                let mut operation = json!({
                    "summary": format!("{} {}", endpoint.method, endpoint.path),
                    "description": format!("Handles {} requests to {}", endpoint.method, endpoint.path),
                    "operationId": format!("{}_{}", method_lower, endpoint.path.replace("/", "_").replace("{", "").replace("}", "").trim_start_matches('_')),
                    "responses": {
                        "200": {
                            "description": "Successful response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": format!("#/components/schemas/{}", endpoint.response_type_name)
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Bad request"
                        },
                        "401": {
                            "description": "Unauthorized"
                        },
                        "403": {
                            "description": "Forbidden"
                        },
                        "500": {
                            "description": "Internal server error"
                        }
                    }
                });

                // Add parameters (path parameters)
                if !endpoint.path_params.is_empty() {
                    let mut parameters = vec![];
                    for (param_name, param_type) in &endpoint.path_params {
                        parameters.push(json!({
                            "name": param_name,
                            "in": "path",
                            "required": true,
                            "description": format!("Path parameter of type {}", param_type),
                            "schema": {
                                "$ref": format!("#/components/schemas/{}", param_type)
                            }
                        }));
                    }
                    operation["parameters"] = json!(parameters);
                }

                // Add request body for non-GET methods
                if endpoint.method != "GET" && endpoint.request_type_name != "()" {
                    operation["requestBody"] = json!({
                        "description": "Request body",
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{}", endpoint.request_type_name)
                                }
                            }
                        }
                    });
                }

                // Add security requirements if auth is required
                if endpoint.auth_required {
                    operation["security"] = json!([{
                        "bearerAuth": []
                    }]);

                    if !endpoint.permissions.is_empty() {
                        operation["x-permissions"] = json!(endpoint.permissions);
                    }
                }

                // Add the operation to the path item
                path_item[method_lower] = operation;
            }

            json!({
                "openapi": "3.0.3",
                "info": {
                    "title": format!("{} REST API", stringify!(#service_name)),
                    "version": "1.0.0",
                    "description": format!("OpenAPI 3.0 specification for the {} service", stringify!(#service_name))
                },
                "paths": paths,
                "components": {
                    "schemas": schemas,
                    "securitySchemes": {
                        "bearerAuth": {
                            "type": "http",
                            "scheme": "bearer",
                            "bearerFormat": "JWT",
                            "description": "JWT token for authentication"
                        }
                    }
                }
            })
        }

        /// Write OpenAPI document to the target directory
        pub fn #openapi_to_file_fn_name() -> std::io::Result<()> {
            let doc = #openapi_fn_name();
            let output_path = #output_path_code;

            // Create parent directories if they don't exist
            if let Some(parent) = std::path::Path::new(&output_path).parent() {
                std::fs::create_dir_all(parent)?;
            }

            let json_string = serde_json::to_string_pretty(&doc)?;
            std::fs::write(&output_path, &json_string)?;

            println!("Generated OpenAPI document at: {}", output_path);

            Ok(())
        }
    }
}

/// Generates code to include schema generation for types when schemars is available
pub fn generate_schema_impl_checks(service_def: &ServiceDefinition) -> TokenStream {
    let mut unique_types = HashMap::new();

    // Collect unique request and response types
    for endpoint in &service_def.endpoints {
        if let Some(request_type) = &endpoint.request_type {
            unique_types.insert(quote!(#request_type).to_string(), quote!(#request_type));
        }

        let response_type = &endpoint.response_type;
        unique_types.insert(quote!(#response_type).to_string(), quote!(#response_type));

        // Add path parameter types
        for path_param in &endpoint.path_params {
            let param_type = &path_param.param_type;
            unique_types.insert(quote!(#param_type).to_string(), quote!(#param_type));
        }
    }

    let type_checks: Vec<TokenStream> = unique_types
        .values()
        .map(|type_tokens| {
            quote! {
                const _: () = {
                    fn _assert_json_schema<T: schemars::JsonSchema>() {}
                    fn _check() {
                        _assert_json_schema::<#type_tokens>();
                    }
                };
            }
        })
        .collect();

    quote! {
        #(#type_checks)*
    }
}
