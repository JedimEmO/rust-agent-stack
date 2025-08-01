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

    // Generate unique function names for each service
    let flatten_fn_name = quote::format_ident!(
        "_flatten_schema_defs_{}",
        service_name.to_string().to_lowercase()
    );
    let update_refs_fn_name = quote::format_ident!(
        "_update_refs_recursive_{}",
        service_name.to_string().to_lowercase()
    );
    let generate_example_fn_name = quote::format_ident!(
        "_generate_example_from_schema_{}",
        service_name.to_string().to_lowercase()
    );

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
                    fn #fn_name() -> (serde_json::Value, std::collections::HashMap<String, serde_json::Value>) {
                        let schema = schemars::schema_for!(#type_tokens);
                        let schema_value = serde_json::to_value(&schema).unwrap_or_else(|_| {
                            serde_json::json!({
                                "type": "object",
                                "description": format!("Schema for {}", #type_name)
                            })
                        });

                        // Extract $defs and flatten them
                        let mut extracted_defs = std::collections::HashMap::new();
                        let flattened_schema = #flatten_fn_name(schema_value, &mut extracted_defs);
                        (flattened_schema, extracted_defs)
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
                    let (schema, defs) = #fn_name();
                    schemas.insert(#type_name.to_string(), schema);
                    // Merge extracted defs into the main schemas collection
                    for (def_name, def_schema) in defs {
                        schemas.insert(def_name, def_schema);
                    }
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
            // Flatten permission groups for OpenRPC documentation
            let permissions = match &method.auth {
                AuthRequirement::Unauthorized => vec![],
                AuthRequirement::WithPermissions(groups) => {
                    // For OpenRPC docs, flatten all permission groups into a single list
                    groups.iter().flatten().cloned().collect()
                }
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

        /// Helper function to extract examples from a JSON schema
        fn #flatten_fn_name(
            mut schema: serde_json::Value,
            extracted_defs: &mut std::collections::HashMap<String, serde_json::Value>
        ) -> serde_json::Value {
            if let Some(obj) = schema.as_object_mut() {
                // If this schema has $defs, extract them
                if let Some(defs) = obj.remove("$defs") {
                    if let Some(defs_obj) = defs.as_object() {
                        for (def_name, def_schema) in defs_obj {
                            // Recursively flatten nested $defs in the extracted definitions
                            let flattened_def = #flatten_fn_name(def_schema.clone(), extracted_defs);
                            extracted_defs.insert(def_name.clone(), flattened_def);
                        }
                    }
                }

                // Update all $ref paths to point to components/schemas
                #update_refs_fn_name(&mut schema);
            }

            schema
        }

        /// Recursively update all $ref paths from #/$defs/ to #/components/schemas/
        fn #update_refs_fn_name(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Object(obj) => {
                    for (key, val) in obj.iter_mut() {
                        if key == "$ref" {
                            if let Some(ref_str) = val.as_str() {
                                if ref_str.starts_with("#/$defs/") {
                                    *val = serde_json::Value::String(
                                        ref_str.replace("#/$defs/", "#/components/schemas/")
                                    );
                                }
                            }
                        } else {
                            #update_refs_fn_name(val);
                        }
                    }
                }
                serde_json::Value::Array(arr) => {
                    for item in arr.iter_mut() {
                        #update_refs_fn_name(item);
                    }
                }
                _ => {}
            }
        }

        /// Generate example value from schema
        fn #generate_example_fn_name(schema: &serde_json::Value, schemas: &std::collections::HashMap<String, serde_json::Value>) -> serde_json::Value {
            // Check if schema has examples field
            if let Some(examples) = schema.get("examples") {
                if let Some(arr) = examples.as_array() {
                    if let Some(first) = arr.first() {
                        return first.clone();
                    }
                }
            }

            // Check if schema has example field (singular)
            if let Some(example) = schema.get("example") {
                return example.clone();
            }

            // Check for $ref
            if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
                if let Some(ref_name) = ref_str.strip_prefix("#/components/schemas/") {
                    if let Some(ref_schema) = schemas.get(ref_name) {
                        return #generate_example_fn_name(ref_schema, schemas);
                    }
                }
            }

            // Handle oneOf/anyOf - pick the first variant
            if let Some(one_of) = schema.get("oneOf").and_then(|v| v.as_array()) {
                if let Some(first_variant) = one_of.first() {
                    return #generate_example_fn_name(first_variant, schemas);
                }
            }
            if let Some(any_of) = schema.get("anyOf").and_then(|v| v.as_array()) {
                if let Some(first_variant) = any_of.first() {
                    return #generate_example_fn_name(first_variant, schemas);
                }
            }

            // Generate based on type
            match schema.get("type").and_then(|v| v.as_str()) {
                Some("string") => serde_json::json!("example_string"),
                Some("number") | Some("integer") => serde_json::json!(42),
                Some("boolean") => serde_json::json!(true),
                Some("array") => {
                    if let Some(items) = schema.get("items") {
                        serde_json::json!([#generate_example_fn_name(items, schemas)])
                    } else {
                        serde_json::json!(["example_item"])
                    }
                }
                Some("object") => {
                    let mut obj = serde_json::Map::new();
                    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                        for (key, prop_schema) in props {
                            obj.insert(key.clone(), #generate_example_fn_name(prop_schema, schemas));
                        }
                        serde_json::json!(obj)
                    } else {
                        serde_json::json!({"example_key": "example_value"})
                    }
                }
                Some("null") => serde_json::json!(null),
                _ => serde_json::json!({"example": "value"})
            }
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
                    // Get the schema for the request type to generate an example
                    let example = if let Some(schema) = schemas.get(&method.request_type_name) {
                        #generate_example_fn_name(schema, &schemas)
                    } else {
                        json!({"example": "value"})
                    };

                    params.push(json!({
                        "name": "params",
                        "description": format!("Request parameters of type {}", method.request_type_name),
                        "required": true,
                        "schema": {
                            "$ref": format!("#/components/schemas/{}", method.request_type_name)
                        },
                        "example": example
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

                // Generate example pairing for the method
                let mut examples = vec![];
                if method.request_type_name != "()" {
                    // Get the schema for the request type to generate an example
                    let request_example = if let Some(schema) = schemas.get(&method.request_type_name) {
                        #generate_example_fn_name(schema, &schemas)
                    } else {
                        json!({"example": "value"})
                    };

                    let response_example = if method.response_type_name != "()" {
                        if let Some(schema) = schemas.get(&method.response_type_name) {
                            #generate_example_fn_name(schema, &schemas)
                        } else {
                            json!({"example": "response"})
                        }
                    } else {
                        json!(null)
                    };

                    examples.push(json!({
                        "name": format!("{}_example", method.name),
                        "description": format!("Example call to {}", method.name),
                        "params": [{"name": "params", "value": request_example}],
                        "result": {"name": "result", "value": response_example}
                    }));
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

                // Add examples if available
                if !examples.is_empty() {
                    if let Some(obj) = method_obj.as_object_mut() {
                        obj.insert("examples".to_string(), json!(examples));
                    }
                }

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
