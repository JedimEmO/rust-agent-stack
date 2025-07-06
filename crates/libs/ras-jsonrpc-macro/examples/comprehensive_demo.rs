//! Comprehensive example showing all features of the jsonrpc_service macro
//!
//! This example demonstrates:
//! - Service without OpenRPC
//! - Service with OpenRPC enabled (default path)
//! - Service with OpenRPC enabled (custom path)
//! - Various authentication requirements
//! - Multiple permission combinations

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
mod basic_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;

    jsonrpc_service!({
        service_name: BasicService,
        methods: [
            UNAUTHORIZED health_check(()) -> StatusResponse,
            WITH_PERMISSIONS(["user.read"]) get_user(UserRequest) -> UserResponse,
        ]
    });
}

// Service with OpenRPC enabled using default path (target/openrpc/{service_name}.json)
mod api_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;

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
}

// Service with OpenRPC enabled using custom output path
mod documented_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;

    jsonrpc_service!({
        service_name: DocumentedService,
        openrpc: { output: "docs/api/service.openrpc.json" },
        methods: [
            UNAUTHORIZED status(()) -> StatusResponse,
            WITH_PERMISSIONS(["service.use"]) process_request(UserRequest) -> UserResponse,
        ]
    });
}

fn main() {
    println!("=== Comprehensive JSON-RPC Service Demo ===\n");

    // Test basic service (no OpenRPC)
    println!("1. Basic Service (no OpenRPC):");
    let basic_builder = basic_service::BasicServiceBuilder::new("/basic");
    let _basic_router = basic_builder.build();
    println!("   ✓ BasicService compiled successfully");
    println!("   ✓ No OpenRPC functions generated\n");

    // Test API service with default OpenRPC
    println!("2. API Service (OpenRPC enabled, default path):");
    let api_builder = api_service::ApiServiceBuilder::new("/api/v1");
    let _api_router = api_builder.build();

    // Generate OpenRPC document
    let openrpc_doc = api_service::generate_apiservice_openrpc();
    println!("   ✓ ApiService compiled successfully");
    println!("   ✓ OpenRPC document generated:");
    println!("     - OpenRPC version: {}", openrpc_doc["openrpc"]);
    println!("     - API title: {}", openrpc_doc["info"]["title"]);
    println!(
        "     - Methods count: {}",
        openrpc_doc["methods"].as_array().unwrap().len()
    );

    // Write to default path
    match api_service::generate_apiservice_openrpc_to_file() {
        Ok(()) => println!("   ✓ Written to: target/openrpc/apiservice.json"),
        Err(e) => println!("   ✗ Error writing file: {}", e),
    }
    println!();

    // Test documented service with custom OpenRPC path
    println!("3. Documented Service (OpenRPC enabled, custom path):");
    let doc_builder = documented_service::DocumentedServiceBuilder::new("/docs/api");
    let _doc_router = doc_builder.build();

    // Generate OpenRPC document
    let doc_openrpc = documented_service::generate_documentedservice_openrpc();
    println!("   ✓ DocumentedService compiled successfully");
    println!("   ✓ OpenRPC document generated with custom path");
    println!(
        "     - Methods count: {}",
        doc_openrpc["methods"].as_array().unwrap().len()
    );

    // Write to custom path
    match documented_service::generate_documentedservice_openrpc_to_file() {
        Ok(()) => println!("   ✓ Written to: docs/api/service.openrpc.json"),
        Err(e) => println!("   ✗ Error writing file: {}", e),
    }
    println!();

    // Show method info
    println!("4. Method Summary:");
    println!("   BasicService:");
    println!("     - health_check: UNAUTHORIZED");
    println!("     - get_user: requires [user.read]");
    println!();
    println!("   ApiService:");
    println!("     - register: UNAUTHORIZED");
    println!("     - authenticated_ping: requires authentication (no specific permissions)");
    println!("     - get_profile: requires [user.read]");
    println!("     - update_profile: requires [user.write]");
    println!("     - admin_action: requires [admin.read, admin.write]");
    println!();
    println!("   DocumentedService:");
    println!("     - status: UNAUTHORIZED");
    println!("     - process_request: requires [service.use]");
}
