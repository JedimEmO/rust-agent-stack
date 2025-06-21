use axum::{Router, http::StatusCode};
use ras_auth_core::{AuthError, AuthProvider, AuthenticatedUser};
use ras_rest_macro::rest_service;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::pin::Pin;

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct TestRequest {}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct TestResponse {}

#[derive(Debug)]
struct CustomError {
    message: String,
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sensitive error: {}", self.message)
    }
}

impl std::error::Error for CustomError {}

// Mock auth provider for testing
#[derive(Clone)]
struct MockAuthProvider;

impl AuthProvider for MockAuthProvider {
    fn authenticate(
        &self,
        authorization: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<AuthenticatedUser, AuthError>> + Send + '_>>
    {
        Box::pin(async move {
            if authorization == "valid-token" {
                let mut permissions = HashSet::new();
                permissions.insert("test".to_string());
                Ok(AuthenticatedUser {
                    user_id: "test-user".to_string(),
                    permissions,
                    metadata: None,
                })
            } else {
                Err(AuthError::InvalidToken)
            }
        })
    }

    fn check_permissions(
        &self,
        user: &AuthenticatedUser,
        required_permissions: &[String],
    ) -> Result<(), AuthError> {
        let missing_permissions: Vec<String> = required_permissions
            .iter()
            .filter(|perm| !user.permissions.contains(*perm))
            .cloned()
            .collect();

        if missing_permissions.is_empty() {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions {
                required: required_permissions.to_vec(),
                has: user.permissions.iter().cloned().collect(),
            })
        }
    }
}

rest_service!({
    service_name: ErrorTestService,
    base_path: "/api",
    endpoints: [
        POST WITH_PERMISSIONS(["test"]) error_test(TestRequest) -> TestResponse
    ]
});

#[tokio::test]
async fn test_error_sanitization() {
    // Create a service that returns a sensitive error
    let service = ErrorTestServiceBuilder::new()
        .auth_provider(MockAuthProvider)
        .post_error_test_handler(|_user, _req| async move {
            Err(Box::new(CustomError {
                message: "This contains sensitive database schema information!".to_string(),
            }) as Box<dyn std::error::Error + Send + Sync>)
        })
        .build();

    let app = Router::new().merge(service);

    // Make a request that will trigger the error
    let client = tower::ServiceExt::oneshot(
        app,
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/error_test")
            .header("Authorization", "Bearer valid-token")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(json!({}).to_string()))
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(client.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = axum::body::to_bytes(client.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: Value = serde_json::from_slice(&body).unwrap();

    // Verify that the error message is generic and doesn't contain sensitive information
    assert_eq!(response["error"], "Internal server error");
    assert!(!response["error"].as_str().unwrap().contains("sensitive"));
    assert!(!response["error"].as_str().unwrap().contains("database"));
    assert!(!response["error"].as_str().unwrap().contains("schema"));
}

#[tokio::test]
async fn test_unauthenticated_error_sanitization() {
    // Create a service with a handler that would succeed, but no auth token provided
    let service = ErrorTestServiceBuilder::new()
        .auth_provider(MockAuthProvider)
        .post_error_test_handler(|_user, _req| async move { Ok(TestResponse {}) })
        .build();

    let app = Router::new().merge(service);

    // Make a request without authentication
    let client = tower::ServiceExt::oneshot(
        app,
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/error_test")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(json!({}).to_string()))
            .unwrap(),
    )
    .await
    .unwrap();

    // Should get authentication error, not internal error details
    assert_eq!(client.status(), StatusCode::UNAUTHORIZED);

    let body = axum::body::to_bytes(client.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: Value = serde_json::from_slice(&body).unwrap();

    // Authentication errors should be generic
    assert_eq!(response["error"], "Missing or invalid Authorization header");
}
