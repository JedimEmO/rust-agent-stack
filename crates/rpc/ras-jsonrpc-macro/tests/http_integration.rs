use rand::Rng;
use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_jsonrpc_macro::jsonrpc_service;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use tokio::net::TcpListener as TokioTcpListener;

// Test data structures for various scenarios
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
    email: String,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ComplexRequest {
    data: Vec<NestedData>,
    metadata: Option<MetadataInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct NestedData {
    id: i32,
    value: String,
    active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct MetadataInfo {
    version: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ProcessingResult {
    processed_count: usize,
    errors: Vec<String>,
    success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}

// Simple test auth provider
struct TestAuthProvider {
    valid_tokens: HashSet<String>,
}

impl TestAuthProvider {
    fn new() -> Self {
        let mut valid_tokens = HashSet::new();
        valid_tokens.insert("valid-admin-token".to_string());
        valid_tokens.insert("valid-user-token".to_string());
        valid_tokens.insert("valid-empty-perms-token".to_string());

        Self { valid_tokens }
    }
}

impl AuthProvider for TestAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            if !self.valid_tokens.contains(&token) {
                return Err(AuthError::InvalidToken);
            }

            let (user_id, permissions) = match token.as_str() {
                "valid-admin-token" => {
                    ("admin-user", vec!["admin".to_string(), "user".to_string()])
                }
                "valid-user-token" => ("regular-user", vec!["user".to_string()]),
                "valid-empty-perms-token" => ("guest-user", vec![]),
                _ => return Err(AuthError::InvalidToken),
            };

            Ok(AuthenticatedUser {
                user_id: user_id.to_string(),
                permissions: permissions.into_iter().collect(),
                metadata: None,
            })
        })
    }
}

// Generate comprehensive test service
jsonrpc_service!({
    service_name: TestService,
    openrpc: true,
    methods: [
        // No auth required
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        UNAUTHORIZED get_public_info(()) -> String,
        UNAUTHORIZED echo_complex(ComplexRequest) -> ComplexRequest,

        // Any valid token required (empty permissions list)
        WITH_PERMISSIONS([]) sign_out(()) -> (),
        WITH_PERMISSIONS([]) get_user_info(()) -> User,
        WITH_PERMISSIONS([]) process_data(Vec<String>) -> ProcessingResult,

        // Specific permissions required
        WITH_PERMISSIONS(["admin"]) delete_everything(()) -> (),
        WITH_PERMISSIONS(["admin"]) create_user(CreateUserRequest) -> User,
        WITH_PERMISSIONS(["admin", "moderator"]) moderate_content(String) -> bool,

        // User permission required
        WITH_PERMISSIONS(["user"]) update_profile(User) -> User,
        WITH_PERMISSIONS(["user"]) get_user_data(i32) -> Option<User>,
    ]
});

async fn create_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let tokio_listener = TokioTcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port");
    let addr = tokio_listener
        .local_addr()
        .expect("Failed to get local addr");
    let base_url = format!("http://127.0.0.1:{}", addr.port());

    let builder = TestServiceBuilder::new("/rpc")
        .auth_provider(TestAuthProvider::new())
        // UNAUTHORIZED handlers
        .sign_in_handler(|request| async move {
            // Simulate authentication logic
            if request.email == "admin@test.com" && request.password == "admin123" {
                Ok(SignInResponse {
                    jwt: "valid-admin-token".to_string(),
                    user_id: "admin-user".to_string(),
                })
            } else if request.email == "user@test.com" && request.password == "user123" {
                Ok(SignInResponse {
                    jwt: "valid-user-token".to_string(),
                    user_id: "regular-user".to_string(),
                })
            } else {
                Err("Invalid credentials".into())
            }
        })
        .get_public_info_handler(|_| async move { Ok("This is public information".to_string()) })
        .echo_complex_handler(|request| async move { Ok(request) })
        // WITH_PERMISSIONS([]) handlers
        .sign_out_handler(|_user, _| async move { Ok(()) })
        .get_user_info_handler(|user, _| async move {
            Ok(User {
                id: Some(123),
                name: format!("User {}", user.user_id),
                email: format!("{}@test.com", user.user_id),
                permissions: user.permissions.into_iter().collect(),
            })
        })
        .process_data_handler(|_user, data| async move {
            Ok(ProcessingResult {
                processed_count: data.len(),
                errors: vec![],
                success: true,
            })
        })
        // WITH_PERMISSIONS(["admin"]) handlers
        .delete_everything_handler(|_user, _| async move { Ok(()) })
        .create_user_handler(|_user, request| async move {
            Ok(User {
                id: Some(rand::thread_rng().gen_range(1000..9999)),
                name: request.name,
                email: request.email,
                permissions: request.permissions,
            })
        })
        .moderate_content_handler(|_user, content| async move { Ok(!content.contains("spam")) })
        // WITH_PERMISSIONS(["user"]) handlers
        .update_profile_handler(|_user, mut user| async move {
            user.id = Some(456);
            Ok(user)
        })
        .get_user_data_handler(|_user, user_id| async move {
            if user_id == 123 {
                Ok(Some(User {
                    id: Some(user_id),
                    name: "Found User".to_string(),
                    email: "found@test.com".to_string(),
                    permissions: vec!["user".to_string()],
                }))
            } else {
                Ok(None)
            }
        });

    let app = builder.build().expect("Failed to build app");

    let handle = tokio::spawn(async move {
        axum::serve(tokio_listener, app)
            .await
            .expect("Server failed");
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    (base_url, handle)
}

async fn make_jsonrpc_request(
    base_url: &str,
    method: &str,
    params: Value,
    token: Option<&str>,
) -> Result<Value, reqwest::Error> {
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });

    let mut request_builder = reqwest::Client::new()
        .post(format!("{}/rpc", base_url))
        .header("Content-Type", "application/json")
        .json(&request_body);

    if let Some(token) = token {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
    }

    let response = request_builder.send().await?;
    let json_response: Value = response.json().await?;
    Ok(json_response)
}

#[tokio::test]
async fn test_unauthorized_methods() {
    let (base_url, _handle) = create_test_server().await;

    // Test sign_in with valid credentials
    let response = make_jsonrpc_request(
        &base_url,
        "sign_in",
        json!({
            "email": "admin@test.com",
            "password": "admin123"
        }),
        None,
    )
    .await
    .unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response.get("error").is_none());

    let result = &response["result"];
    assert_eq!(result["jwt"], "valid-admin-token");
    assert_eq!(result["user_id"], "admin-user");

    // Test sign_in with invalid credentials
    let response = make_jsonrpc_request(
        &base_url,
        "sign_in",
        json!({
            "email": "wrong@test.com",
            "password": "wrong"
        }),
        None,
    )
    .await
    .unwrap();

    assert!(response.get("error").is_some());

    // Test get_public_info
    let response = make_jsonrpc_request(&base_url, "get_public_info", json!(()), None)
        .await
        .unwrap();

    assert_eq!(response["result"], "This is public information");

    // Test echo_complex
    let complex_data = json!({
        "data": [
            {"id": 1, "value": "test", "active": true},
            {"id": 2, "value": "test2", "active": false}
        ],
        "metadata": {
            "version": "1.0",
            "tags": ["test", "demo"]
        }
    });

    let response = make_jsonrpc_request(&base_url, "echo_complex", complex_data.clone(), None)
        .await
        .unwrap();

    assert_eq!(response["result"], complex_data);
}

#[tokio::test]
async fn test_authentication_required_methods() {
    let (base_url, _handle) = create_test_server().await;

    // Test without token - should fail
    let response = make_jsonrpc_request(&base_url, "sign_out", json!(()), None)
        .await
        .unwrap();

    assert!(response.get("error").is_some());
    let error = &response["error"];
    assert_eq!(error["code"], -32001); // Custom auth error code

    // Test with valid token - should succeed
    let response =
        make_jsonrpc_request(&base_url, "sign_out", json!(()), Some("valid-admin-token"))
            .await
            .unwrap();

    assert!(response.get("error").is_none());
    assert_eq!(response["result"], json!(()));

    // Test get_user_info with valid token
    let response = make_jsonrpc_request(
        &base_url,
        "get_user_info",
        json!(()),
        Some("valid-user-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_none());
    let result = &response["result"];
    assert_eq!(result["name"], "User regular-user");
    assert_eq!(result["email"], "regular-user@test.com");

    // Test process_data
    let response = make_jsonrpc_request(
        &base_url,
        "process_data",
        json!(["item1", "item2", "item3"]),
        Some("valid-empty-perms-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_none());
    let result = &response["result"];
    assert_eq!(result["processed_count"], 3);
    assert_eq!(result["success"], true);
}

#[tokio::test]
async fn test_admin_permission_methods() {
    let (base_url, _handle) = create_test_server().await;

    // Test with user token (insufficient permissions) - should fail
    let response = make_jsonrpc_request(
        &base_url,
        "delete_everything",
        json!(()),
        Some("valid-user-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_some());
    let error = &response["error"];
    assert_eq!(error["code"], -32002); // Insufficient permissions error

    // Test with admin token - should succeed
    let response = make_jsonrpc_request(
        &base_url,
        "delete_everything",
        json!(()),
        Some("valid-admin-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_none());

    // Test create_user with admin token
    let response = make_jsonrpc_request(
        &base_url,
        "create_user",
        json!({
            "name": "New User",
            "email": "new@test.com",
            "permissions": ["user"]
        }),
        Some("valid-admin-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_none());
    let result = &response["result"];
    assert_eq!(result["name"], "New User");
    assert_eq!(result["email"], "new@test.com");
    assert!(result["id"].as_i64().unwrap() >= 1000);
}

#[tokio::test]
async fn test_user_permission_methods() {
    let (base_url, _handle) = create_test_server().await;

    // Test with empty permissions token - should fail
    let response = make_jsonrpc_request(
        &base_url,
        "update_profile",
        json!({
            "name": "Updated User",
            "email": "updated@test.com",
            "permissions": []
        }),
        Some("valid-empty-perms-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_some());

    // Test with user token - should succeed
    let response = make_jsonrpc_request(
        &base_url,
        "update_profile",
        json!({
            "name": "Updated User",
            "email": "updated@test.com",
            "permissions": []
        }),
        Some("valid-user-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_none());
    let result = &response["result"];
    assert_eq!(result["name"], "Updated User");
    assert_eq!(result["id"], 456);

    // Test get_user_data with existing user
    let response = make_jsonrpc_request(
        &base_url,
        "get_user_data",
        json!(123),
        Some("valid-user-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_none());
    let result = &response["result"];
    assert_eq!(result["name"], "Found User");

    // Test get_user_data with non-existing user
    let response = make_jsonrpc_request(
        &base_url,
        "get_user_data",
        json!(999),
        Some("valid-user-token"),
    )
    .await
    .unwrap();

    assert!(response.get("error").is_none());
    assert_eq!(response["result"], json!(null));
}

#[tokio::test]
async fn test_invalid_requests() {
    let (base_url, _handle) = create_test_server().await;

    // Test method not found
    let response = make_jsonrpc_request(&base_url, "non_existent_method", json!(()), None)
        .await
        .unwrap();

    assert!(response.get("error").is_some());
    let error = &response["error"];
    assert_eq!(error["code"], -32601); // Method not found

    // Test invalid JSON-RPC format (missing jsonrpc field)
    let invalid_request = json!({
        "method": "sign_in",
        "params": {},
        "id": 1
    });

    let response = reqwest::Client::new()
        .post(format!("{}/rpc", base_url))
        .header("Content-Type", "application/json")
        .json(&invalid_request)
        .send()
        .await
        .unwrap();

    let json_response: Value = response.json().await.unwrap();
    assert!(json_response.get("error").is_some());

    // Test invalid parameters for a method
    let response = make_jsonrpc_request(&base_url, "sign_in", json!("invalid_params"), None)
        .await
        .unwrap();

    assert!(response.get("error").is_some());
}

#[tokio::test]
async fn test_concurrent_requests() {
    let (base_url, _handle) = create_test_server().await;

    // Test multiple concurrent requests
    let mut handles = vec![];

    for _ in 0..10 {
        let base_url = base_url.clone();
        let handle = tokio::spawn(async move {
            make_jsonrpc_request(&base_url, "get_public_info", json!(()), None).await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(handles).await;

    // All requests should succeed
    for result in results {
        let response = result.unwrap().unwrap();
        assert_eq!(response["result"], "This is public information");
    }
}

#[tokio::test]
async fn test_openrpc_generation() {
    // Test that OpenRPC document is generated correctly
    let openrpc_doc = generate_testservice_openrpc();

    assert_eq!(openrpc_doc["openrpc"], "1.3.2");
    assert_eq!(openrpc_doc["info"]["title"], "TestService JSON-RPC API");

    let methods = openrpc_doc["methods"].as_array().unwrap();
    assert_eq!(methods.len(), 11); // We have 11 methods defined

    // Check that unauthorized methods don't have authentication metadata
    let sign_in_method = methods.iter().find(|m| m["name"] == "sign_in").unwrap();
    assert!(sign_in_method.get("x-authentication").is_none());

    // Check that admin methods have correct permissions
    let delete_method = methods
        .iter()
        .find(|m| m["name"] == "delete_everything")
        .unwrap();
    assert_eq!(delete_method["x-authentication"]["required"], true);
    assert_eq!(delete_method["x-permissions"][0], "admin");

    // Check that methods with multiple permissions are correct
    let moderate_method = methods
        .iter()
        .find(|m| m["name"] == "moderate_content")
        .unwrap();
    let permissions = moderate_method["x-permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&json!("admin")));
    assert!(permissions.contains(&json!("moderator")));
}

#[cfg(feature = "client")]
#[tokio::test]
async fn test_client_generation() {
    // Test that client generation compiles and produces valid API
    let client_result = TestServiceClientBuilder::new()
        .server_url("http://localhost:9999/rpc")
        .with_timeout(std::time::Duration::from_millis(1000))
        .build();

    assert!(client_result.is_ok());

    let mut client = client_result.unwrap();
    client.set_bearer_token(Some("test-token"));
    assert_eq!(client.bearer_token(), Some("test-token"));

    // Try to call a method - this should fail with connection error but proves the API works
    let request = SignInRequest {
        email: "test@example.com".to_string(),
        password: "password".to_string(),
    };

    let result = client.sign_in(request.clone()).await;
    // Should get a connection error since server doesn't exist
    assert!(result.is_err());

    // Test timeout version
    let result = client
        .sign_in_with_timeout(request, std::time::Duration::from_millis(100))
        .await;
    assert!(result.is_err());
}
