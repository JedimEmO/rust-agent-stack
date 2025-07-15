use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use serde::{Deserialize, Serialize};

// Test types
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct TestRequest {
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct TestResponse {
    result: String,
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

// Generate a test service
mod test_service {
    use super::*;
    use ras_jsonrpc_macro::jsonrpc_service;

    jsonrpc_service!({
        service_name: TestService,
        methods: [
            UNAUTHORIZED method_one(TestRequest) -> TestResponse,
            WITH_PERMISSIONS(["admin"]) method_two(TestRequest) -> TestResponse,
            WITH_PERMISSIONS([]) method_three(TestRequest) -> TestResponse,
        ]
    });
}

#[test]
fn test_error_on_missing_handlers() {
    use test_service::*;

    // Create a service builder with only one handler configured
    let builder = TestServiceBuilder::new("/api")
        .auth_provider(TestAuthProvider)
        .method_one_handler(|_request| async move {
            Ok(TestResponse {
                result: "one".to_string(),
            })
        });
    // Note: method_two_handler and method_three_handler are NOT configured

    // This should return an error with missing handlers
    let result = builder.build();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Cannot build service: the following handlers are not configured: method_two, method_three"
    );
}

#[test]
fn test_error_on_all_handlers_missing() {
    use test_service::*;

    // Create a service builder with no handlers configured
    let builder = TestServiceBuilder::new("/api")
        .auth_provider(TestAuthProvider);

    // This should return an error with all handlers missing
    let result = builder.build();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Cannot build service: the following handlers are not configured: method_one, method_two, method_three"
    );
}

#[test]
fn test_success_with_all_handlers_configured() {
    use test_service::*;

    // Create a service builder with all handlers configured
    let builder = TestServiceBuilder::new("/api")
        .auth_provider(TestAuthProvider)
        .method_one_handler(|_request| async move {
            Ok(TestResponse {
                result: "one".to_string(),
            })
        })
        .method_two_handler(|_user, _request| async move {
            Ok(TestResponse {
                result: "two".to_string(),
            })
        })
        .method_three_handler(|_user, _request| async move {
            Ok(TestResponse {
                result: "three".to_string(),
            })
        });

    // This should succeed
    let result = builder.build();
    assert!(result.is_ok());
}