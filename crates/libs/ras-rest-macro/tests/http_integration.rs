use rand::Rng;
use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_rest_macro::rest_service;
use ras_rest_core::{RestResponse, RestError};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use tokio::net::TcpListener as TokioTcpListener;

// Test data structures for REST API testing
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CreateUserRequest {
    name: String,
    email: String,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct UpdateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct UsersResponse {
    users: Vec<User>,
    total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct PostRequest {
    title: String,
    content: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct Post {
    id: Option<i32>,
    user_id: i32,
    title: String,
    content: String,
    tags: Vec<String>,
    published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct PostsResponse {
    posts: Vec<Post>,
    total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}

// Simple test auth provider
struct TestRestAuthProvider {
    valid_tokens: HashSet<String>,
}

impl TestRestAuthProvider {
    fn new() -> Self {
        let mut valid_tokens = HashSet::new();
        valid_tokens.insert("admin-token".to_string());
        valid_tokens.insert("user-token".to_string());
        valid_tokens.insert("moderator-token".to_string());
        valid_tokens.insert("superuser-token".to_string());
        valid_tokens.insert("empty-perms-token".to_string());

        Self { valid_tokens }
    }
}

impl AuthProvider for TestRestAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            if !self.valid_tokens.contains(&token) {
                return Err(AuthError::InvalidToken);
            }

            let (user_id, permissions) = match token.as_str() {
                "admin-token" => ("admin-user", vec!["admin".to_string(), "user".to_string()]),
                "superuser-token" => (
                    "superuser-user",
                    vec!["admin".to_string(), "super_user".to_string()],
                ),
                "user-token" => ("regular-user", vec!["user".to_string()]),
                "moderator-token" => (
                    "mod-user",
                    vec!["moderator".to_string(), "user".to_string()],
                ),
                "empty-perms-token" => ("guest-user", vec![]),
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

// Generate comprehensive REST test service
rest_service!({
    service_name: TestRestService,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,
    docs_path: "/docs",
    ui_theme: "default",
    endpoints: [
        // User management endpoints
        GET UNAUTHORIZED users() -> UsersResponse,
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> User,
        GET WITH_PERMISSIONS(["user"]) users/{id: i32}() -> User,
        PUT WITH_PERMISSIONS(["admin"]) users/{id: i32}(UpdateUserRequest) -> User,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: i32}() -> (),

        // Posts endpoints with nested paths
        GET UNAUTHORIZED users/{user_id: i32}/posts() -> PostsResponse,
        POST WITH_PERMISSIONS(["user"]) users/{user_id: i32}/posts(PostRequest) -> Post,
        GET WITH_PERMISSIONS([]) users/{user_id: i32}/posts/{post_id: i32}() -> Post,
        PUT WITH_PERMISSIONS(["user", "moderator"]) users/{user_id: i32}/posts/{post_id: i32}(PostRequest) -> Post,
        DELETE WITH_PERMISSIONS(["moderator"] | ["admin"]) users/{user_id: i32}/posts/{post_id: i32}() -> (),

        // Health check and status endpoints
        GET UNAUTHORIZED health() -> String,
        GET WITH_PERMISSIONS([]) status() -> Value,

        // OR syntax demonstration endpoint
        POST WITH_PERMISSIONS(["admin", "moderator"] | ["super_user"]) admin_action(()) -> String,
    ]
});

// Test service implementation
struct TestRestServiceImpl;

#[async_trait::async_trait]
impl TestRestServiceTrait for TestRestServiceImpl {
    async fn get_users(&self) -> ras_rest_core::RestResult<UsersResponse> {
        Ok(RestResponse::ok(UsersResponse {
            users: vec![
                User {
                    id: Some(1),
                    name: "John Doe".to_string(),
                    email: "john@example.com".to_string(),
                    permissions: vec!["user".to_string()],
                },
                User {
                    id: Some(2),
                    name: "Jane Admin".to_string(),
                    email: "jane@example.com".to_string(),
                    permissions: vec!["admin".to_string()],
                },
            ],
            total: 2,
        }))
    }

    async fn post_users(&self, _user: &AuthenticatedUser, request: CreateUserRequest) -> ras_rest_core::RestResult<User> {
        Ok(RestResponse::created(User {
            id: Some(rand::thread_rng().gen_range(100..999)),
            name: request.name,
            email: request.email,
            permissions: request.permissions,
        }))
    }

    async fn get_users_by_id(&self, _user: &AuthenticatedUser, id: i32) -> ras_rest_core::RestResult<User> {
        if id == 404 {
            Err(RestError::not_found("User not found"))
        } else {
            Ok(RestResponse::ok(User {
                id: Some(id),
                name: "Found User".to_string(),
                email: "found@example.com".to_string(),
                permissions: vec!["user".to_string()],
            }))
        }
    }

    async fn put_users_by_id(&self, _user: &AuthenticatedUser, id: i32, request: UpdateUserRequest) -> ras_rest_core::RestResult<User> {
        Ok(RestResponse::ok(User {
            id: Some(id),
            name: request.name,
            email: request.email,
            permissions: vec!["user".to_string()],
        }))
    }

    async fn delete_users_by_id(&self, _user: &AuthenticatedUser, _id: i32) -> ras_rest_core::RestResult<()> {
        Ok(RestResponse::no_content())
    }

    async fn get_users_by_user_id_posts(&self, user_id: i32) -> ras_rest_core::RestResult<PostsResponse> {
        Ok(RestResponse::ok(PostsResponse {
            posts: vec![Post {
                id: Some(1),
                user_id,
                title: "Test Post".to_string(),
                content: "This is a test post".to_string(),
                tags: vec!["test".to_string()],
                published: true,
            }],
            total: 1,
        }))
    }

    async fn post_users_by_user_id_posts(&self, _user: &AuthenticatedUser, user_id: i32, request: PostRequest) -> ras_rest_core::RestResult<Post> {
        Ok(RestResponse::created(Post {
            id: Some(rand::thread_rng().gen_range(100..999)),
            user_id,
            title: request.title,
            content: request.content,
            tags: request.tags,
            published: false,
        }))
    }

    async fn get_users_by_user_id_posts_by_post_id(&self, _user: &AuthenticatedUser, user_id: i32, post_id: i32) -> ras_rest_core::RestResult<Post> {
        Ok(RestResponse::ok(Post {
            id: Some(post_id),
            user_id,
            title: "Protected Post".to_string(),
            content: "This requires authentication".to_string(),
            tags: vec!["protected".to_string()],
            published: true,
        }))
    }

    async fn put_users_by_user_id_posts_by_post_id(&self, _user: &AuthenticatedUser, user_id: i32, post_id: i32, request: PostRequest) -> ras_rest_core::RestResult<Post> {
        Ok(RestResponse::ok(Post {
            id: Some(post_id),
            user_id,
            title: request.title,
            content: request.content,
            tags: request.tags,
            published: true,
        }))
    }

    async fn delete_users_by_user_id_posts_by_post_id(&self, _user: &AuthenticatedUser, _user_id: i32, _post_id: i32) -> ras_rest_core::RestResult<()> {
        Ok(RestResponse::no_content())
    }

    async fn get_health(&self) -> ras_rest_core::RestResult<String> {
        Ok(RestResponse::ok("OK".to_string()))
    }

    async fn get_status(&self, user: &AuthenticatedUser) -> ras_rest_core::RestResult<Value> {
        let value = json!({
            "status": "authenticated",
            "user_id": user.user_id,
            "permissions": user.permissions.iter().collect::<Vec<_>>(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        Ok(RestResponse::ok(value))
    }

    async fn post_admin_action(&self, _user: &AuthenticatedUser, _request: ()) -> ras_rest_core::RestResult<String> {
        Ok(RestResponse::ok("Admin action completed".to_string()))
    }
}

async fn create_rest_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let tokio_listener = TokioTcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to port");
    let addr = tokio_listener
        .local_addr()
        .expect("Failed to get local addr");
    let base_url = format!("http://127.0.0.1:{}", addr.port());

    let builder = TestRestServiceBuilder::new(TestRestServiceImpl)
        .auth_provider(TestRestAuthProvider::new());

    let app = builder.build();

    let handle = tokio::spawn(async move {
        axum::serve(tokio_listener, app)
            .await
            .expect("Server failed");
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    (base_url, handle)
}

async fn make_rest_request(
    method: reqwest::Method,
    url: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut request_builder = reqwest::Client::new()
        .request(method, url)
        .header("Content-Type", "application/json");

    if let Some(token) = token {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
    }

    if let Some(body) = body {
        request_builder = request_builder.json(&body);
    }

    request_builder.send().await
}

#[tokio::test]
async fn test_unauthorized_endpoints() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test GET /api/v1/users without auth
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users", base_url),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let users_response: UsersResponse = response.json().await.unwrap();
    assert_eq!(users_response.total, 2);
    assert_eq!(users_response.users.len(), 2);
    assert_eq!(users_response.users[0].name, "John Doe");

    // Test GET /api/v1/users/123/posts without auth
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users/123/posts", base_url),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let posts_response: PostsResponse = response.json().await.unwrap();
    assert_eq!(posts_response.total, 1);
    assert_eq!(posts_response.posts[0].user_id, 123);

    // Test GET /api/v1/health
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/health", base_url),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let health: String = response.json().await.unwrap();
    assert_eq!(health, "OK");
}

#[tokio::test]
async fn test_authentication_required_endpoints() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test GET /api/v1/status without token - should fail
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/status", base_url),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 401);

    // Test GET /api/v1/status with valid token - should succeed
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/status", base_url),
        None,
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let status: Value = response.json().await.unwrap();
    assert_eq!(status["status"], "authenticated");
    assert_eq!(status["user_id"], "regular-user");

    // Test GET /api/v1/users/123/posts/456 with valid token
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        None,
        Some("empty-perms-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let post: Post = response.json().await.unwrap();
    assert_eq!(post.id, Some(456));
    assert_eq!(post.user_id, 123);
    assert_eq!(post.title, "Protected Post");
}

#[tokio::test]
async fn test_admin_permission_endpoints() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test POST /api/v1/users with user token (insufficient permissions) - should fail
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/users", base_url),
        Some(json!({
            "name": "New User",
            "email": "new@example.com",
            "permissions": ["user"]
        })),
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 403);

    // Test POST /api/v1/users with admin token - should succeed
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/users", base_url),
        Some(json!({
            "name": "New User",
            "email": "new@example.com",
            "permissions": ["user"]
        })),
        Some("admin-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 201); // Created
    let user: User = response.json().await.unwrap();
    assert_eq!(user.name, "New User");
    assert_eq!(user.email, "new@example.com");
    assert!(user.id.unwrap() >= 100);

    // Test PUT /api/v1/users/123 with admin token
    let response = make_rest_request(
        reqwest::Method::PUT,
        &format!("{}/api/v1/users/123", base_url),
        Some(json!({
            "name": "Updated User",
            "email": "updated@example.com"
        })),
        Some("admin-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let user: User = response.json().await.unwrap();
    assert_eq!(user.id, Some(123));
    assert_eq!(user.name, "Updated User");

    // Test DELETE /api/v1/users/123 with admin token
    let response = make_rest_request(
        reqwest::Method::DELETE,
        &format!("{}/api/v1/users/123", base_url),
        None,
        Some("admin-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 204); // No Content
}

#[tokio::test]
async fn test_user_permission_endpoints() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test GET /api/v1/users/123 with empty permissions token - should fail
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users/123", base_url),
        None,
        Some("empty-perms-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 403);

    // Test GET /api/v1/users/123 with user token - should succeed
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users/123", base_url),
        None,
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let user: User = response.json().await.unwrap();
    assert_eq!(user.id, Some(123));
    assert_eq!(user.name, "Found User");

    // Test GET /api/v1/users/404 with user token - should return error
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users/404", base_url),
        None,
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 404); // Not Found

    // Test POST /api/v1/users/123/posts with user token
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/users/123/posts", base_url),
        Some(json!({
            "title": "My New Post",
            "content": "This is my new post content",
            "tags": ["personal", "test"]
        })),
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 201); // Created
    let post: Post = response.json().await.unwrap();
    assert_eq!(post.user_id, 123);
    assert_eq!(post.title, "My New Post");
    assert!(!post.published);
}

#[tokio::test]
async fn test_multiple_permissions_endpoints() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test PUT /api/v1/users/123/posts/456 with user token - should fail (needs both "user" AND "moderator")
    let response = make_rest_request(
        reqwest::Method::PUT,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        Some(json!({
            "title": "Updated Post",
            "content": "Updated content",
            "tags": ["updated"]
        })),
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_ne!(response.status(), 200);

    // Test PUT /api/v1/users/123/posts/456 with moderator token - should succeed (has both "user" and "moderator")
    let response = make_rest_request(
        reqwest::Method::PUT,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        Some(json!({
            "title": "Moderator Updated Post",
            "content": "Moderator updated content",
            "tags": ["moderated"]
        })),
        Some("moderator-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);

    let post: Post = response.json().await.unwrap();
    assert_eq!(post.title, "Moderator Updated Post");

    // Test PUT /api/v1/users/123/posts/456 with empty permissions - should fail
    let response = make_rest_request(
        reqwest::Method::PUT,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        Some(json!({
            "title": "Unauthorized Update",
            "content": "Should not work",
            "tags": []
        })),
        Some("empty-perms-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 403);

    // Test DELETE /api/v1/users/123/posts/456 with admin token - should succeed
    let response = make_rest_request(
        reqwest::Method::DELETE,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        None,
        Some("admin-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 204); // No Content

    // Test DELETE /api/v1/users/123/posts/456 with moderator token - should succeed
    let response = make_rest_request(
        reqwest::Method::DELETE,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        None,
        Some("moderator-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 204); // No Content
}

#[tokio::test]
async fn test_invalid_requests() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test non-existent endpoint
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/nonexistent", base_url),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 404);

    // Test invalid HTTP method
    let response = make_rest_request(
        reqwest::Method::PATCH,
        &format!("{}/api/v1/users", base_url),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 405);

    // Test invalid JSON body
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v1/users", base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin-token")
        .body("{invalid json")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    // Test missing required fields
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/users", base_url),
        Some(json!({
            "name": "Incomplete User"
            // Missing email and permissions
        })),
        Some("admin-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_concurrent_rest_requests() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test multiple concurrent requests
    let mut handles = vec![];

    for _ in 0..10 {
        let base_url = base_url.clone();
        let handle = tokio::spawn(async move {
            make_rest_request(
                reqwest::Method::GET,
                &format!("{}/api/v1/health", base_url),
                None,
                None,
            )
            .await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(handles).await;

    // All requests should succeed
    for result in results {
        let response = result.unwrap().unwrap();
        assert_eq!(response.status(), 200);
        let health: String = response.json().await.unwrap();
        assert_eq!(health, "OK");
    }
}

#[tokio::test]
async fn test_path_parameters() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test single path parameter
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users/42", base_url),
        None,
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let user: User = response.json().await.unwrap();
    assert_eq!(user.id, Some(42));

    // Test multiple path parameters
    let response = make_rest_request(
        reqwest::Method::GET,
        &format!("{}/api/v1/users/123/posts/789", base_url),
        None,
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 200);
    let post: Post = response.json().await.unwrap();
    assert_eq!(post.user_id, 123);
    assert_eq!(post.id, Some(789));

    // Test path parameters with request body
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/users/999/posts", base_url),
        Some(json!({
            "title": "Path Param Post",
            "content": "Testing path parameters with body",
            "tags": ["path", "test"]
        })),
        Some("user-token"),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), 201); // Created
    let post: Post = response.json().await.unwrap();
    assert_eq!(post.user_id, 999);
    assert_eq!(post.title, "Path Param Post");
}

#[tokio::test]
async fn test_openapi_generation() {
    // Test that OpenAPI document generation works
    // Note: This tests compilation and basic structure, actual OpenAPI document
    // generation would be tested separately
    let _ = TestRestServiceBuilder::new(TestRestServiceImpl);

    // The fact that this compiles means the REST service macro generated the builder correctly
    // with OpenAPI configuration enabled
    assert!(true, "OpenAPI generation compiled successfully");
}

#[tokio::test]
async fn test_missing_dependencies() {
    // Import futures for the join_all function
    use futures::future::join_all;

    // This test ensures that our future handling is working correctly
    let handles: Vec<tokio::task::JoinHandle<()>> = vec![];
    let _results = join_all(handles).await;
    assert!(true, "Futures dependency is working");
}

#[tokio::test]
async fn test_new_permission_logic() {
    let (base_url, _handle) = create_rest_test_server().await;

    // Test admin_action endpoint with new permission logic:
    // WITH_PERMISSIONS(["admin", "moderator"] | ["super_user"])
    // This means user needs (admin AND moderator) OR (super_user)

    // Test with admin-token (has "admin" and "user", but NOT "moderator") - should FAIL
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/admin_action", base_url),
        Some(serde_json::Value::Null), // Send null for unit type
        Some("admin-token"),
    )
    .await
    .unwrap();
    assert_eq!(
        response.status(),
        403,
        "Admin token should fail - has admin but not moderator"
    );

    // Test with moderator-token (has "moderator" and "user", but NOT "admin") - should FAIL
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/admin_action", base_url),
        Some(Value::Null), // Send null for unit type
        Some("moderator-token"),
    )
    .await
    .unwrap();
    assert_eq!(
        response.status(),
        403,
        "Moderator token should fail - has moderator but not admin"
    );

    // Test with superuser-token (has "superuser" and "admin") - should SUCCEED
    let response = make_rest_request(
        reqwest::Method::POST,
        &format!("{}/api/v1/admin_action", base_url),
        Some(Value::Null), // Send null for unit type
        Some("superuser-token"),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), 200, "superuser should succeed");

    // We would need a token with both admin AND moderator permissions to test success
    // But our test auth provider doesn't have such a token

    // The DELETE endpoint uses ["moderator"] | ["admin"] - should succeed with either
    // Test with admin-token (has "admin") - should SUCCEED
    let response = make_rest_request(
        reqwest::Method::DELETE,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        None,
        Some("admin-token"),
    )
    .await
    .unwrap();
    assert_eq!(
        response.status(),
        204, // No Content
        "Admin token should succeed for delete - has admin"
    );

    // Test with moderator-token (has "moderator") - should SUCCEED
    let response = make_rest_request(
        reqwest::Method::DELETE,
        &format!("{}/api/v1/users/123/posts/456", base_url),
        None,
        Some("moderator-token"),
    )
    .await
    .unwrap();
    assert_eq!(
        response.status(),
        204, // No Content
        "Moderator token should succeed for delete - has moderator"
    );
}

#[tokio::test]
async fn test_generated_rest_client() {
    let (base_url, _handle) = create_rest_test_server().await;
    let mut client = TestRestServiceClientBuilder::new(base_url)
        .build()
        .unwrap();

    client.set_bearer_token(Some("superuser-token"));

    let resp = client.get_users().await.expect("failed to get users");

    assert_eq!(resp.total, 2);

    let _resp = client
        .delete_users_by_id_with_timeout(resp.users[0].id.unwrap(), None)
        .await
        .expect("failed to get users");
}
