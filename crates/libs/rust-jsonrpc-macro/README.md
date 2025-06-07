# rust-jsonrpc-macro

Procedural macros for generating type-safe JSON-RPC services with authentication and axum integration.

## Overview

This crate provides the `jsonrpc_service!` procedural macro that generates type-safe JSON-RPC services with built-in authentication, authorization, and seamless axum integration. It transforms a declarative service definition into a fully functional JSON-RPC server with compile-time safety guarantees.

## Features

- ✅ **Declarative Service Definition**: Clean, readable syntax for defining JSON-RPC methods
- ✅ **Authentication Integration**: Built-in support for `UNAUTHORIZED` and `WITH_PERMISSIONS` methods
- ✅ **Type Safety**: Compile-time validation of request/response types
- ✅ **Axum Integration**: Generates standard axum `Router` for easy composition
- ✅ **Builder Pattern**: Ergonomic service configuration using the `bon` crate
- ✅ **Async Support**: Full async/await support throughout
- ✅ **JSON-RPC 2.0 Compliant**: Complete protocol compliance with proper error handling

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-jsonrpc-macro = "0.1.0"
rust-jsonrpc-core = "0.1.0"  # For AuthProvider trait
axum = "0.8"                  # For web server integration
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

### 1. Define Your Service

```rust
use rust_jsonrpc_macro::jsonrpc_service;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SignInRequest {
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct SignInResponse {
    jwt: String,
    user_id: String,
}

jsonrpc_service!({
    service_name: MyService,
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS(["user"]) get_profile(()) -> UserProfile,
        WITH_PERMISSIONS(["admin"]) delete_user(UserId) -> (),
    ]
});
```

### 2. Implement an Auth Provider

```rust
use rust_jsonrpc_core::{AuthProvider, AuthenticatedUser, AuthFuture};
use std::collections::HashSet;

struct MyAuthProvider;

impl AuthProvider for MyAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            // Validate JWT token (simplified)
            if token.starts_with("valid_") {
                let mut permissions = HashSet::new();
                permissions.insert("user".to_string());
                
                if token.contains("admin") {
                    permissions.insert("admin".to_string());
                }
                
                Ok(AuthenticatedUser {
                    user_id: "user123".to_string(),
                    permissions,
                    metadata: None,
                })
            } else {
                Err(rust_jsonrpc_core::AuthError::InvalidToken)
            }
        })
    }
}
```

### 3. Build and Run Your Service

```rust
use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .nest("/api", 
            MyServiceBuilder::new("/rpc")
                .auth_provider(MyAuthProvider)
                .sign_in_handler(|request| async move {
                    // Validate credentials
                    Ok(SignInResponse {
                        jwt: "valid_user_token".to_string(),
                        user_id: "123".to_string(),
                    })
                })
                .get_profile_handler(|user, _request| async move {
                    // User is already authenticated and authorized
                    Ok(UserProfile {
                        name: format!("User {}", user.user_id),
                        email: "user@example.com".to_string(),
                    })
                })
                .delete_user_handler(|user, user_id| async move {
                    // User is authenticated and has "admin" permission
                    println!("Admin {} deleting user {:?}", user.user_id, user_id);
                    Ok(())
                })
                .build()
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
```

## Macro Syntax

### Service Definition

```rust
jsonrpc_service!({
    service_name: ServiceName,  // Name of the generated service
    methods: [
        // Method definitions...
    ]
});
```

### Method Definitions

#### Unauthorized Methods
```rust
UNAUTHORIZED method_name(RequestType) -> ResponseType,
```
- No authentication required
- Handler signature: `Fn(RequestType) -> Future<Output = Result<ResponseType, Error>>`

#### Permission-Based Methods
```rust
WITH_PERMISSIONS(["perm1", "perm2"]) method_name(RequestType) -> ResponseType,
```
- Requires valid authentication
- Checks for specified permissions
- Handler signature: `Fn(AuthenticatedUser, RequestType) -> Future<Output = Result<ResponseType, Error>>`

#### Empty Permissions (Any Valid Token)
```rust
WITH_PERMISSIONS([]) method_name(RequestType) -> ResponseType,
```
- Requires valid authentication
- No specific permissions required
- Handler signature: `Fn(AuthenticatedUser, RequestType) -> Future<Output = Result<ResponseType, Error>>`

## Generated Code

The macro generates:

### Service Builder
```rust
pub struct MyServiceBuilder {
    // Internal fields...
}

impl MyServiceBuilder {
    pub fn new(base_url: impl Into<String>) -> Self { /* ... */ }
    pub fn auth_provider<T: AuthProvider>(self, provider: T) -> Self { /* ... */ }
    pub fn method_name_handler<F, Fut>(self, handler: F) -> Self { /* ... */ }
    pub fn build(self) -> axum::Router { /* ... */ }
}
```

### Request Handling
- Automatic JSON-RPC request/response parsing
- Authentication token extraction from `Authorization` header
- Permission validation
- Error handling with proper JSON-RPC error codes

## Authentication Flow

### 1. Token Extraction
The generated service automatically extracts Bearer tokens from the `Authorization` header:
```
Authorization: Bearer <token>
```

### 2. Method Routing
- `UNAUTHORIZED` methods bypass authentication
- `WITH_PERMISSIONS` methods require valid authentication and authorization

### 3. Error Responses
Authentication failures return proper JSON-RPC 2.0 error responses:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Authentication required"
  },
  "id": 1
}
```

## JSON-RPC Client Examples

### Sign In (Unauthorized)
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "sign_in",
    "params": {
      "email": "user@example.com",
      "password": "secret"
    },
    "id": 1
  }'
```

### Get Profile (With Authentication)
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer valid_user_token" \
  -d '{
    "jsonrpc": "2.0",
    "method": "get_profile",
    "params": {},
    "id": 2
  }'
```

### Delete User (Admin Permission Required)
```bash
curl -X POST http://localhost:3000/api/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer valid_admin_token" \
  -d '{
    "jsonrpc": "2.0",
    "method": "delete_user",
    "params": {"id": "user456"},
    "id": 3
  }'
```

## Error Handling

The macro generates comprehensive error handling:

- **Parse Errors**: Invalid JSON (-32700)
- **Invalid Request**: Malformed JSON-RPC (-32600)  
- **Method Not Found**: Unknown method (-32601)
- **Invalid Params**: Type mismatch (-32602)
- **Authentication Required**: Missing/invalid token (-32001)
- **Insufficient Permissions**: Missing permissions (-32002)
- **Internal Errors**: Handler errors (-32603)

## Integration

This crate works seamlessly with:

- [`rust-jsonrpc-core`](../rust-jsonrpc-core) - Authentication traits and types
- [`rust-jsonrpc-types`](../rust-jsonrpc-types) - JSON-RPC protocol types
- [`axum`](https://crates.io/crates/axum) - Web framework
- [`bon`](https://crates.io/crates/bon) - Builder pattern generation

## Examples

See the [`examples/`](../../examples/) directory for complete working examples:

- [`basic-service`](../../examples/basic-service) - Complete service with authentication
- [`usage.rs`](examples/usage.rs) - Standalone usage example

## License

This project is licensed under the MIT License.