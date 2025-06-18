use axum::{Router, routing::get};
use ras_jsonrpc_core::{AuthFuture, AuthProvider, AuthenticatedUser};
use ras_jsonrpc_macro::jsonrpc_service;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, JsonSchema)]
pub enum SignInRequest {
    WithCredentials { username: String, password: String },
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub enum SignInResponse {
    Success { jwt: String },
    Failure { msg: String },
}

impl Default for SignInResponse {
    fn default() -> Self {
        Self::Success { jwt: String::new() }
    }
}

// Example auth provider
pub struct MyAuthProvider;

impl AuthProvider for MyAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            // Simple example - in real implementation, validate JWT
            if token == "valid_token" {
                let mut permissions = HashSet::new();
                permissions.insert("user".to_string());

                Ok(AuthenticatedUser {
                    user_id: "user123".to_string(),
                    permissions,
                    metadata: None,
                })
            } else if token == "admin_token" {
                let mut permissions = HashSet::new();
                permissions.insert("user".to_string());
                permissions.insert("admin".to_string());

                Ok(AuthenticatedUser {
                    user_id: "admin123".to_string(),
                    permissions,
                    metadata: None,
                })
            } else {
                Err(ras_jsonrpc_core::AuthError::InvalidToken)
            }
        })
    }
}

jsonrpc_service!({
    service_name: MyService,
    openrpc: true,
    explorer: true,
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS([]) sign_out(()) -> (),
        WITH_PERMISSIONS(["admin"]) delete_everything(()) -> (),
    ]
});

async fn handler() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rpc_router = MyServiceBuilder::new("/rpc")
        .auth_provider(MyAuthProvider)
        .sign_in_handler(|request| async move {
            match request {
                SignInRequest::WithCredentials { username, password } => {
                    if username == "admin" && password == "secret" {
                        Ok(SignInResponse::Success {
                            jwt: "admin_token".to_string(),
                        })
                    } else if username == "user" && password == "password" {
                        Ok(SignInResponse::Success {
                            jwt: "valid_token".to_string(),
                        })
                    } else {
                        Ok(SignInResponse::Failure {
                            msg: "Invalid credentials".to_string(),
                        })
                    }
                }
            }
        })
        .sign_out_handler(|user, _request| async move {
            tracing::info!("User {} signed out", user.user_id);
            Ok(())
        })
        .delete_everything_handler(|user, _request| async move {
            tracing::warn!("Admin {} is deleting everything!", user.user_id);
            Ok(())
        })
        .build();

    let app = Router::new().route("/", get(handler)).nest(
        "/api",
        rpc_router,
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    println!("JSON-RPC endpoint: http://0.0.0.0:3000/api/rpc");
    println!("JSON-RPC Explorer: http://0.0.0.0:3000/api/explorer");
    println!("");
    println!("Example credentials:");
    println!("  - Username: user, Password: password (basic user)");
    println!("  - Username: admin, Password: secret (admin user)");
    axum::serve(listener, app).await.unwrap();
}
