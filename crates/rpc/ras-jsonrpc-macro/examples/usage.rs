use ras_jsonrpc_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use ras_jsonrpc_macro::jsonrpc_service;
use serde::{Deserialize, Serialize};

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

struct MyAuthProvider;

impl AuthProvider for MyAuthProvider {
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

#[tokio::main]
async fn main() {
    println!("Building JSON-RPC service with the generated macro...");

    let _router = MyServiceBuilder::new("/api/v1")
        .auth_provider(MyAuthProvider)
        .sign_in_handler(|request| async move {
            println!("Handling sign_in: {:?}", request);
            Ok(SignInResponse {
                jwt: "generated-jwt-token".to_string(),
                user_id: "user-123".to_string(),
            })
        })
        .sign_out_handler(|user, _request| async move {
            println!("User {} signing out", user.user_id);
            Ok(())
        })
        .delete_everything_handler(|user, _request| async move {
            println!("User {} deleting everything (admin action)", user.user_id);
            Ok(())
        })
        .build();

    println!("JSON-RPC service router created successfully!");
    println!("The router can be used with axum to serve HTTP requests.");

    // In a real application, you would do:
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    // axum::serve(listener, router).await.unwrap();
}
