//! OpenRPC document generation module
//!
//! This module provides functionality to generate OpenRPC specification documents
//! from the jsonrpc_service macro definitions.

use crate::{AuthRequirement, OpenRpcConfig, ServiceDefinition};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

/// Generates OpenRPC document creation code
pub fn generate_openrpc_code(
    service_def: &ServiceDefinition,
    config: &OpenRpcConfig,
) -> TokenStream {
    let service_name = &service_def.service_name;
    let openrpc_fn_name = quote::format_ident!(
        "generate_{}_openrpc",
        service_name.to_string().to_lowercase()
    );
    let openrpc_to_file_fn_name = quote::format_ident!(
        "generate_{}_openrpc_to_file",
        service_name.to_string().to_lowercase()
    );
    let method_info_struct_name = quote::format_ident!("{}OpenRpcMethodInfo", service_name);

    // Generate the output path based on config
    let output_path_code = match config {
        OpenRpcConfig::Enabled => {
            let service_name_lower = service_name.to_string().to_lowercase();
            quote! {
                format!("target/openrpc/{}.json", #service_name_lower)
            }
        }
        OpenRpcConfig::WithPath(path) => {
            quote! {
                #path.to_string()
            }
        }
    };

    // Collect unique types for schema generation
    let mut unique_types = std::collections::HashMap::new();
    for method in &service_def.methods {
        let request_type = &method.request_type;
        let response_type = &method.response_type;

        let request_type_str = quote!(#request_type).to_string();
        let response_type_str = quote!(#response_type).to_string();

        unique_types.insert(request_type_str, quote!(#request_type));
        unique_types.insert(response_type_str, quote!(#response_type));
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
                        "description": "Unit type"
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

    // Generate method info structs
    let method_infos: Vec<TokenStream> = service_def
        .methods
        .iter()
        .map(|method| {
            let method_name = method.name.to_string();
            let auth_required = matches!(method.auth, AuthRequirement::WithPermissions(_));
            let permissions = match &method.auth {
                AuthRequirement::Unauthorized => vec![],
                AuthRequirement::WithPermissions(perms) => perms.clone(),
            };

            let request_type = &method.request_type;
            let response_type = &method.response_type;

            quote! {
                #method_info_struct_name {
                    name: #method_name.to_string(),
                    auth_required: #auth_required,
                    permissions: vec![#(#permissions.to_string()),*],
                    request_type_name: stringify!(#request_type).to_string(),
                    response_type_name: stringify!(#response_type).to_string(),
                }
            }
        })
        .collect();

    quote! {
        #[derive(serde::Serialize)]
        struct #method_info_struct_name {
            name: String,
            auth_required: bool,
            permissions: Vec<String>,
            request_type_name: String,
            response_type_name: String,
        }

        // Generate schema functions for each type
        #(#schema_fns)*

        /// Generate OpenRPC document for this service
        pub fn #openrpc_fn_name() -> serde_json::Value {
            use serde_json::json;
            use schemars::{schema_for, JsonSchema};
            use std::collections::HashMap;

            let methods = vec![
                #(#method_infos),*
            ];

            // Generate schemas for all unique types
            let mut schemas = HashMap::new();

            // Insert all the generated schemas
            #(#schema_insertions)*

            let openrpc_methods: Vec<serde_json::Value> = methods.iter().map(|method| {
                let mut params = vec![];

                // Add request parameter only if not unit type
                if method.request_type_name != "()" {
                    params.push(json!({
                        "name": "params",
                        "description": format!("Request parameters of type {}", method.request_type_name),
                        "required": true,
                        "schema": {
                            "$ref": format!("#/components/schemas/{}", method.request_type_name)
                        }
                    }));
                }

                let mut extensions: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();

                if method.auth_required {
                    extensions.insert("x-authentication".to_string(), json!({
                        "required": true,
                        "type": "bearer"
                    }));

                    if !method.permissions.is_empty() {
                        extensions.insert("x-permissions".to_string(), json!(method.permissions));
                    }
                }

                let mut method_obj = json!({
                    "name": method.name,
                    "description": format!("Calls the {} method", method.name),
                    "params": params,
                    "result": {
                        "name": "result",
                        "description": format!("Response of type {}", method.response_type_name),
                        "schema": {
                            "$ref": format!("#/components/schemas/{}", method.response_type_name)
                        }
                    }
                });

                // Add extensions to the method object
                if let Some(obj) = method_obj.as_object_mut() {
                    for (key, value) in extensions {
                        obj.insert(key, value);
                    }
                }

                method_obj
            }).collect();

            json!({
                "openrpc": "1.3.2",
                "info": {
                    "title": format!("{} JSON-RPC API", stringify!(#service_name)),
                    "version": "1.0.0",
                    "description": format!("OpenRPC specification for the {} service", stringify!(#service_name))
                },
                "methods": openrpc_methods,
                "components": {
                    "schemas": schemas,
                    "errors": {
                        "ParseError": {
                            "code": -32700,
                            "message": "Parse error"
                        },
                        "InvalidRequest": {
                            "code": -32600,
                            "message": "Invalid Request"
                        },
                        "MethodNotFound": {
                            "code": -32601,
                            "message": "Method not found"
                        },
                        "InvalidParams": {
                            "code": -32602,
                            "message": "Invalid params"
                        },
                        "InternalError": {
                            "code": -32603,
                            "message": "Internal error"
                        },
                        "AuthenticationRequired": {
                            "code": -32001,
                            "message": "Authentication required"
                        },
                        "InsufficientPermissions": {
                            "code": -32002,
                            "message": "Insufficient permissions"
                        },
                        "TokenExpired": {
                            "code": -32003,
                            "message": "Token expired"
                        }
                    }
                }
            })
        }

        /// Write OpenRPC document to the target directory
        pub fn #openrpc_to_file_fn_name() -> std::io::Result<()> {
            let doc = #openrpc_fn_name();
            let output_path = #output_path_code;

            // Create parent directories if they don't exist
            if let Some(parent) = std::path::Path::new(&output_path).parent() {
                std::fs::create_dir_all(parent)?;
            }

            let json_string = serde_json::to_string_pretty(&doc)?;
            std::fs::write(&output_path, &json_string)?;

            println!("Generated OpenRPC document at: {}", output_path);

            Ok(())
        }
    }
}

/// Generates code to include schema generation for types when schemars is available
pub fn generate_schema_impl_checks(service_def: &ServiceDefinition) -> TokenStream {
    let mut unique_types = HashMap::new();

    // Collect unique request and response types
    for method in &service_def.methods {
        let request_type = &method.request_type;
        let response_type = &method.response_type;

        unique_types.insert(quote!(#request_type).to_string(), quote!(#request_type));
        unique_types.insert(quote!(#response_type).to_string(), quote!(#response_type));
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
