use crate::bruno::{BrunoAuth, BrunoCollection, BrunoEnvironment, BrunoRequest};
use crate::cli::Args;
use crate::error::ToolError;
use openrpc_types::{
    ContentDescriptorOrReference, ContentDescriptorSchema, Method, MethodOrReference, OpenRpc,
    Schema, SchemaType,
};
use serde_json::{Map, Value};
use tokio::fs;

pub struct OpenRpcToBrunoConverter {
    args: Args,
}

impl OpenRpcToBrunoConverter {
    pub fn new(args: Args) -> Self {
        Self { args }
    }

    pub async fn convert(&self) -> Result<(), ToolError> {
        // Step 1: Parse OpenRPC specification
        let openrpc = self.parse_openrpc().await?;

        // Step 2: Validate output directory
        self.prepare_output_directory().await?;

        // Step 3: Generate Bruno collection structure
        self.generate_bruno_collection(&openrpc).await?;

        Ok(())
    }

    async fn parse_openrpc(&self) -> Result<OpenRpc, ToolError> {
        if self.args.verbose {
            println!("📖 Reading OpenRPC file...");
        }

        let content =
            fs::read_to_string(&self.args.input)
                .await
                .map_err(|e| ToolError::InputFileRead {
                    path: self.args.input.clone(),
                    source: e,
                })?;

        let openrpc: OpenRpc = if self.args.input.extension().and_then(|s| s.to_str())
            == Some("yaml")
            || self.args.input.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            // Parse YAML (we'll need to add serde_yaml dependency for this)
            return Err(ToolError::UnsupportedFeature(
                "YAML parsing not yet implemented. Please use JSON format.".to_string(),
            ));
        } else {
            serde_json::from_str(&content)?
        };

        if self.args.verbose {
            println!("✅ Successfully parsed OpenRPC specification");
            println!("   Title: {}", openrpc.info.title);
            println!("   Version: {}", openrpc.info.version);
            println!("   Methods: {}", openrpc.methods.len());
        }

        // Basic validation
        if openrpc.methods.is_empty() {
            return Err(ToolError::NoMethodsDefined);
        }

        Ok(openrpc)
    }

    async fn prepare_output_directory(&self) -> Result<(), ToolError> {
        if self.args.output.exists() {
            if !self.args.force {
                return Err(ToolError::OutputDirExists {
                    path: self.args.output.clone(),
                });
            }
            if self.args.verbose {
                println!("🗑️  Removing existing output directory...");
            }
            fs::remove_dir_all(&self.args.output).await.map_err(|e| {
                ToolError::OutputDirCreate {
                    path: self.args.output.clone(),
                    source: e,
                }
            })?;
        }

        if self.args.verbose {
            println!("📁 Creating output directory...");
        }

        fs::create_dir_all(&self.args.output)
            .await
            .map_err(|e| ToolError::OutputDirCreate {
                path: self.args.output.clone(),
                source: e,
            })?;

        // Create environments subdirectory
        let env_dir = self.args.output.join("environments");
        fs::create_dir_all(&env_dir)
            .await
            .map_err(|e| ToolError::OutputDirCreate {
                path: env_dir.clone(),
                source: e,
            })?;

        Ok(())
    }

    async fn generate_bruno_collection(&self, openrpc: &OpenRpc) -> Result<(), ToolError> {
        // Step 1: Generate bruno.json
        self.generate_collection_metadata(openrpc).await?;

        // Step 2: Generate environment file
        self.generate_environment_file(openrpc).await?;

        // Step 3: Generate request files for each method
        self.generate_method_files(openrpc).await?;

        Ok(())
    }

    async fn generate_collection_metadata(&self, openrpc: &OpenRpc) -> Result<(), ToolError> {
        if self.args.verbose {
            println!("📝 Generating collection metadata...");
        }

        let collection_name = self
            .args
            .name
            .clone()
            .unwrap_or_else(|| openrpc.info.title.clone());

        let collection = BrunoCollection {
            name: collection_name,
            ..Default::default()
        };

        let content = serde_json::to_string_pretty(&collection)?;
        let path = self.args.output.join("bruno.json");

        fs::write(&path, content)
            .await
            .map_err(|e| ToolError::BrunoFileWrite {
                path: path.clone(),
                source: e,
            })?;

        Ok(())
    }

    async fn generate_environment_file(&self, openrpc: &OpenRpc) -> Result<(), ToolError> {
        if self.args.verbose {
            println!("🌍 Generating environment file...");
        }

        let mut environment = BrunoEnvironment::new("default".to_string());

        // Add base URL from args or try to extract from OpenRPC servers
        if let Some(base_url) = &self.args.base_url {
            environment.add_variable("base_url", base_url);
        } else if let Some(servers) = &openrpc.servers {
            if !servers.is_empty() {
                environment.add_variable("base_url", &servers[0].url);
            }
        } else {
            // Default to localhost
            environment.add_variable("base_url", "http://localhost:3000");
        }

        // Add common variables
        environment.add_variable("api_path", "/api/v1/rpc");
        environment.add_variable("auth_token", "PUT YOUR TOKEN HERE");

        let content = environment.to_bru_format();
        let path = self.args.output.join("environments").join("default.bru");

        fs::write(&path, content)
            .await
            .map_err(|e| ToolError::BrunoFileWrite {
                path: path.clone(),
                source: e,
            })?;

        Ok(())
    }

    async fn generate_method_files(&self, openrpc: &OpenRpc) -> Result<(), ToolError> {
        if self.args.verbose {
            println!("🔧 Generating method files...");
        }

        for (index, method_ref) in openrpc.methods.iter().enumerate() {
            match method_ref {
                MethodOrReference::Method(method) => {
                    self.generate_method_file(method, index as u32 + 1).await?;
                }
                MethodOrReference::Reference(_) => {
                    return Err(ToolError::UnsupportedFeature(
                        "Method references are not yet supported".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    async fn generate_method_file(&self, method: &Method, sequence: u32) -> Result<(), ToolError> {
        if self.args.verbose {
            println!("  📄 Generating method: {}", method.name);
        }

        // Create JSON-RPC request body
        let request_body = self.create_jsonrpc_request_body(method)?;

        // Create Bruno request
        let mut bruno_request = BrunoRequest::new(
            method.name.clone(),
            "post".to_string(),
            "{{base_url}}{{api_path}}".to_string(),
        )
        .with_sequence(sequence)
        .with_json_body(serde_json::to_string_pretty(&request_body)?);

        // Add Content-Type header
        bruno_request.add_header("Content-Type", "application/json");

        // Check if method requires authentication based on extensions
        if method.extensions.contains_key("x-authentication")
            || method.extensions.contains_key("x-permissions")
        {
            bruno_request = bruno_request.with_auth(BrunoAuth::Bearer {
                token: "{{auth_token}}".to_string(),
            });
        }

        let content = bruno_request.to_bru_format();
        let path = self.args.output.join(format!("{}.bru", method.name));

        fs::write(&path, content)
            .await
            .map_err(|e| ToolError::BrunoFileWrite {
                path: path.clone(),
                source: e,
            })?;

        Ok(())
    }

    fn create_jsonrpc_request_body(&self, method: &Method) -> Result<Value, ToolError> {
        let mut request = Map::new();
        request.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
        request.insert("method".to_string(), Value::String(method.name.clone()));
        request.insert("id".to_string(), Value::Number(1.into()));

        // Generate example parameters
        let params = self.generate_example_params(method)?;
        if !params.is_null() {
            request.insert("params".to_string(), params);
        }

        Ok(Value::Object(request))
    }

    fn generate_example_params(&self, method: &Method) -> Result<Value, ToolError> {
        if method.params.is_empty() {
            return Ok(Value::Null);
        }

        let mut param_values = Map::new();

        for param_ref in &method.params {
            match param_ref {
                ContentDescriptorOrReference::ContentDescriptor(content_desc) => {
                    let example_value = self.generate_example_value(&content_desc.schema)?;
                    param_values.insert(content_desc.name.clone(), example_value);
                }
                ContentDescriptorOrReference::Reference(_) => {
                    return Err(ToolError::UnsupportedFeature(
                        "Parameter references are not yet supported".to_string(),
                    ));
                }
            }
        }

        Ok(Value::Object(param_values))
    }

    fn generate_example_value(&self, schema: &ContentDescriptorSchema) -> Result<Value, ToolError> {
        match schema {
            ContentDescriptorSchema::Schema(schema) => self.generate_example_from_schema(schema),
            ContentDescriptorSchema::Reference(_) => Err(ToolError::UnsupportedFeature(
                "Schema references are not yet supported".to_string(),
            )),
        }
    }

    fn generate_example_from_schema(&self, schema: &Schema) -> Result<Value, ToolError> {
        // Simple example generation based on schema type
        // This could be extended to be more sophisticated
        match &schema.schema_type {
            Some(SchemaType::String) => Ok(Value::String("example_string".to_string())),
            Some(SchemaType::Number) => Ok(Value::Number(42.into())),
            Some(SchemaType::Integer) => Ok(Value::Number(42.into())),
            Some(SchemaType::Boolean) => Ok(Value::Bool(true)),
            Some(SchemaType::Array) => Ok(Value::Array(vec![Value::String(
                "example_item".to_string(),
            )])),
            Some(SchemaType::Object) => {
                let mut obj = Map::new();
                if let Some(props) = &schema.properties {
                    for (key, _) in props.iter().take(3) {
                        // Limit to first 3 properties
                        obj.insert(key.clone(), Value::String("example_value".to_string()));
                    }
                } else {
                    obj.insert(
                        "example_key".to_string(),
                        Value::String("example_value".to_string()),
                    );
                }
                Ok(Value::Object(obj))
            }
            Some(SchemaType::Null) => Ok(Value::Null),
            None => Ok(Value::String("any_value".to_string())),
        }
    }
}
