use rust_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use rust_jsonrpc_macro::jsonrpc_service;
use serde::{Deserialize, Serialize};

// Test types for requests and responses
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignInRequest {
    email: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignInResponse {
    jwt: String,
    user_id: String,
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

// Generate the service using our macro
jsonrpc_service!({
    service_name: MyService,
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS([]) sign_out(()) -> (),
        WITH_PERMISSIONS(["admin"]) delete_everything(()) -> (),
    ]
});

#[tokio::test]
async fn test_macro_generation() {
    // Create a service builder
    let builder = MyServiceBuilder::new("/api/v1")
        .auth_provider(TestAuthProvider)
        .sign_in_handler(|_request| async move {
            Ok(SignInResponse {
                jwt: "test-jwt".to_string(),
                user_id: "123".to_string(),
            })
        })
        .sign_out_handler(|_user, _request| async move { Ok(()) })
        .delete_everything_handler(|_user, _request| async move { Ok(()) });

    // Build the router
    let _router = builder.build();

    // Test passes if it compiles
    println!("Macro generated code successfully!");
}

#[test]
fn test_macro_compilation() {
    // This test ensures the macro generates syntactically correct code
    // that can be compiled
}
