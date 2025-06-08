//! Comprehensive example showing all features of the jsonrpc_service macro
//!
//! This example demonstrates:
//! - Service without OpenRPC
//! - Service with OpenRPC enabled (default path)
//! - Service with OpenRPC enabled (custom path)
//! - Various authentication requirements
//! - Multiple permission combinations

use ras_jsonrpc_macro::jsonrpc_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Common types used across all services
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AdminAction {
    pub action: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StatusResponse {
    pub success: bool,
    pub message: String,
}

// Service without OpenRPC generation
jsonrpc_service!({
    service_name: BasicService,
    methods: [
        UNAUTHORIZED health_check(()) -> StatusResponse,
        WITH_PERMISSIONS(["user.read"]) get_user(UserRequest) -> UserResponse,
    ]
});

// Service with OpenRPC enabled using default path (target/openrpc/{service_name}.json)
jsonrpc_service!({
    service_name: ApiService,
    openrpc: true,
    methods: [
        UNAUTHORIZED register(UserRequest) -> UserResponse,
        WITH_PERMISSIONS([]) authenticated_ping(()) -> StatusResponse,
        WITH_PERMISSIONS(["user.read"]) get_profile(()) -> UserResponse,
        WITH_PERMISSIONS(["user.write"]) update_profile(UserRequest) -> UserResponse,
        WITH_PERMISSIONS(["admin.read", "admin.write"]) admin_action(AdminAction) -> StatusResponse,
    ]
});

// Service with OpenRPC enabled using custom output path
jsonrpc_service!({
    service_name: DocumentedService,
    openrpc: { output: "docs/api/service.openrpc.json" },
    methods: [
        UNAUTHORIZED status(()) -> StatusResponse,
        WITH_PERMISSIONS(["service.use"]) process_request(UserRequest) -> UserResponse,
    ]
});

fn main() {
    println!("=== Comprehensive JSON-RPC Service Demo ===\n");

    // Test basic service (no OpenRPC)
    println!("1. Basic Service (no OpenRPC):");
    let basic_builder = BasicServiceBuilder::new("/basic");
    let _basic_router = basic_builder.build();
    println!("   ✓ BasicService compiled successfully");
    println!("   ✓ No OpenRPC functions generated\n");

    // Test API service with default OpenRPC
    println!("2. API Service (OpenRPC enabled, default path):");
    let api_builder = ApiServiceBuilder::new("/api/v1");
    let _api_router = api_builder.build();

    // Generate OpenRPC document
    let openrpc_doc = generate_apiservice_openrpc();
    println!("   ✓ ApiService compiled successfully");
    println!("   ✓ OpenRPC document generated:");
    println!("     - OpenRPC version: {}", openrpc_doc["openrpc"]);
    println!("     - API title: {}", openrpc_doc["info"]["title"]);
    println!(
        "     - Methods count: {}",
        openrpc_doc["methods"].as_array().unwrap().len()
    );

    // Write to default path
    match generate_apiservice_openrpc_to_file() {
        Ok(()) => println!("   ✓ Written to: target/openrpc/apiservice.json"),
        Err(e) => println!("   ✗ Error writing file: {}", e),
    }
    println!();

    // Test documented service with custom path
    println!("3. Documented Service (OpenRPC enabled, custom path):");
    let doc_builder = DocumentedServiceBuilder::new("/docs/api");
    let _doc_router = doc_builder.build();

    // Generate OpenRPC document
    let doc_openrpc = generate_documentedservice_openrpc();
    println!("   ✓ DocumentedService compiled successfully");
    println!("   ✓ OpenRPC document generated:");
    println!("     - API title: {}", doc_openrpc["info"]["title"]);
    println!(
        "     - Methods count: {}",
        doc_openrpc["methods"].as_array().unwrap().len()
    );

    // Write to custom path
    match generate_documentedservice_openrpc_to_file() {
        Ok(()) => println!("   ✓ Written to: docs/api/service.openrpc.json"),
        Err(e) => println!("   ✗ Error writing file: {}", e),
    }
    println!();

    // Demonstrate method authentication analysis
    println!("4. Authentication Analysis:");
    let methods = openrpc_doc["methods"].as_array().unwrap();

    for method in methods {
        let name = method["name"].as_str().unwrap();
        let has_auth = method.get("x-authentication").is_some();
        let permissions = method
            .get("x-permissions")
            .and_then(|p| p.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        if has_auth {
            if permissions.is_empty() {
                println!("   - {}: Requires authentication (any valid token)", name);
            } else {
                println!("   - {}: Requires permissions: {:?}", name, permissions);
            }
        } else {
            println!("   - {}: Public (no authentication required)", name);
        }
    }

    println!("\n=== Demo completed successfully! ===");
}
