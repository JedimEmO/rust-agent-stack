use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use serde::{Deserialize, Serialize};

// Test types for requests and responses
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct SignInRequest {
    email: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct SignInResponse {
    jwt: String,
    user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CreateUserRequest {
    name: String,
    role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct User {
    id: String,
    name: String,
    role: String,
}

// Simple auth provider for testing
struct TestAuthProvider;

impl AuthProvider for TestAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            if token == "valid-token" {
                Ok(AuthenticatedUser {
                    user_id: "test-user".to_string(),
                    permissions: ["admin".to_string()].into_iter().collect(),
                    metadata: None,
                })
            } else {
                Err(AuthError::InvalidToken)
            }
        })
    }
}

mod basic_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;
    
    // Generate the service using our macro
    jsonrpc_service!({
        service_name: MyService,
        methods: [
            UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
            WITH_PERMISSIONS(["admin"]) create_user(CreateUserRequest) -> User,
            WITH_PERMISSIONS([]) get_profile(()) -> User,
        ]
    });
}

#[tokio::test]
async fn test_macro_generates_code() {
    use basic_service::*;
    
    // Create a service builder
    let builder = MyServiceBuilder::new("/api/v1")
        .auth_provider(TestAuthProvider)
        .sign_in_handler(|_request| async move {
            Ok(SignInResponse {
                jwt: "test-jwt".to_string(),
                user_id: "123".to_string(),
            })
        })
        .create_user_handler(|_user, request| async move {
            Ok(User {
                id: "new-id".to_string(),
                name: request.name,
                role: request.role,
            })
        })
        .get_profile_handler(|user, _request| async move {
            Ok(User {
                id: user.user_id.clone(),
                name: "Test User".to_string(),
                role: "user".to_string(),
            })
        });

    // Build the router (this ensures all generated code compiles)
    let _router = builder.build();

    println!("Macro generated code successfully!");
}

// Generate a service with OpenRPC enabled
mod openrpc_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;
    
    jsonrpc_service!({
        service_name: OpenRpcService,
        openrpc: true,
        methods: [
            UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
            WITH_PERMISSIONS(["admin"]) create_user(CreateUserRequest) -> User,
            WITH_PERMISSIONS([]) sign_out(()) -> (),
        ]
    });
}

// Generate a service with custom OpenRPC output path
mod custom_path_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;
    
    jsonrpc_service!({
        service_name: CustomPathService,
        openrpc: { output: "custom/path/service.json" },
        methods: [
            UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
            WITH_PERMISSIONS(["admin"]) delete_everything(()) -> (),
        ]
    });
}

#[tokio::test]
async fn test_openrpc_generation() {
    use openrpc_service::*;
    
    // Create a service builder with OpenRPC enabled
    let builder = OpenRpcServiceBuilder::new("/api/v1")
        .auth_provider(TestAuthProvider)
        .sign_in_handler(|_request| async move {
            Ok(SignInResponse {
                jwt: "test-jwt".to_string(),
                user_id: "123".to_string(),
            })
        })
        .create_user_handler(|_user, request| async move {
            Ok(User {
                id: "new-id".to_string(),
                name: request.name,
                role: request.role,
            })
        })
        .sign_out_handler(|_user, _request| async move { Ok(()) });

    // Build the router
    let _router = builder.build();

    // Generate and write OpenRPC document
    let openrpc_doc = generate_openrpcservice_openrpc();
    assert_eq!(openrpc_doc["openrpc"], "1.3.2");
    assert_eq!(openrpc_doc["info"]["title"], "OpenRpcService JSON-RPC API");

    // Check that methods are present
    let methods = openrpc_doc["methods"].as_array().unwrap();
    assert_eq!(methods.len(), 3);

    // Check sign_in method (unauthorized)
    let sign_in_method = methods.iter().find(|m| m["name"] == "sign_in").unwrap();
    assert!(sign_in_method.get("x-authentication").is_none());

    // Check create_user method (requires admin permission)
    let create_user_method = methods.iter().find(|m| m["name"] == "create_user").unwrap();
    assert_eq!(create_user_method["x-authentication"]["required"], true);
    assert_eq!(create_user_method["x-permissions"][0], "admin");

    // Test writing to file
    assert!(generate_openrpcservice_openrpc_to_file().is_ok());

    println!("OpenRPC generation test passed!");
}

#[tokio::test]
async fn test_custom_openrpc_path() {
    use custom_path_service::*;
    
    // Create a service builder
    let builder = CustomPathServiceBuilder::new("/api/v2")
        .auth_provider(TestAuthProvider)
        .sign_in_handler(|_request| async move {
            Ok(SignInResponse {
                jwt: "test-jwt".to_string(),
                user_id: "123".to_string(),
            })
        })
        .delete_everything_handler(|_user, _request| async move { Ok(()) });

    // Build the router
    let _router = builder.build();

    // Generate OpenRPC document
    let openrpc_doc = generate_custompathservice_openrpc();
    assert_eq!(openrpc_doc["openrpc"], "1.3.2");

    println!("Custom OpenRPC path test passed!");
}