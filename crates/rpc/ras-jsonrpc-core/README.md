# ras-jsonrpc-core

Core authentication and authorization traits for JSON-RPC services.

## Overview

This crate provides the foundational authentication and authorization traits used by the `ras-jsonrpc-macro` procedural macro to generate type-safe JSON-RPC services with axum integration. It defines the `AuthProvider` trait that enables flexible authentication mechanisms while maintaining a consistent interface.

## Features

- ✅ **Async Authentication**: Full async/await support for authentication operations
- ✅ **Permission-Based Authorization**: Fine-grained permission checking
- ✅ **Flexible Auth Providers**: Support for JWT, API keys, or custom authentication
- ✅ **Comprehensive Error Handling**: Detailed error types for all authentication scenarios
- ✅ **Extension Traits**: Optional authentication helpers
- ✅ **Integration Ready**: Re-exports JSON-RPC types for convenience

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-jsonrpc-core = "0.1.0"
```

### Implementing an Auth Provider

```rust
use rust_jsonrpc_core::{AuthProvider, AuthenticatedUser, AuthFuture, AuthError};
use std::collections::HashSet;

struct JwtAuthProvider {
    secret_key: String,
}

impl AuthProvider for JwtAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            // Validate JWT token (simplified example)
            if self.validate_jwt(&token)? {
                let claims = self.decode_jwt(&token)?;
                
                Ok(AuthenticatedUser {
                    user_id: claims.user_id,
                    permissions: claims.permissions,
                    metadata: Some(serde_json::json!({
                        "iat": claims.issued_at,
                        "exp": claims.expires_at
                    })),
                })
            } else {
                Err(AuthError::InvalidToken)
            }
        })
    }
}
```

### Using with Permissions

```rust
use rust_jsonrpc_core::{AuthProvider, AuthProviderExt};

async fn example_usage() {
    let auth_provider = JwtAuthProvider::new("secret".to_string());
    
    // Authenticate and authorize in one step
    let user = auth_provider
        .authenticate_and_authorize(
            "jwt-token".to_string(),
            vec!["admin".to_string()]
        )
        .await?;
    
    // Optional authentication
    let maybe_user = auth_provider
        .authenticate_optional(Some("jwt-token".to_string()))
        .await?;
}
```

## Authentication Flow

### 1. Token Validation
```rust
// Extract token from request headers
let token = extract_bearer_token(&headers)?;

// Validate token
let user = auth_provider.authenticate(token).await?;
```

### 2. Permission Checking
```rust
// Check if user has required permissions
auth_provider.check_permissions(
    &user,
    &["admin".to_string(), "write".to_string()]
)?;
```

### 3. Combined Auth + Authz
```rust
// Authenticate and authorize in one step
let user = auth_provider.authenticate_and_authorize(
    token,
    vec!["admin".to_string()]
).await?;
```

## Error Types

The crate provides comprehensive error handling:

```rust
use rust_jsonrpc_core::AuthError;

match auth_result {
    Err(AuthError::InvalidToken) => {
        // Token is malformed or invalid
    }
    Err(AuthError::TokenExpired) => {
        // Token has expired
    }
    Err(AuthError::InsufficientPermissions { required, has }) => {
        // User lacks required permissions
        eprintln!("Need {:?}, but user has {:?}", required, has);
    }
    Err(AuthError::AuthenticationRequired) => {
        // No token provided but authentication required
    }
    Err(AuthError::Internal(msg)) => {
        // Internal authentication error
        eprintln!("Auth error: {}", msg);
    }
    Ok(user) => {
        // Authentication successful
        println!("Authenticated user: {}", user.user_id);
    }
}
```

## Types

### AuthenticatedUser

Represents a successfully authenticated user:

```rust
pub struct AuthenticatedUser {
    /// Unique identifier for the user
    pub user_id: String,
    
    /// Set of permissions granted to this user
    pub permissions: HashSet<String>,
    
    /// Optional additional metadata about the user
    pub metadata: Option<serde_json::Value>,
}
```

### Type Aliases

- `AuthResult<T>` - Result type for authentication operations
- `AuthFuture<'a, T>` - Boxed future for async authentication

## Integration with rust-jsonrpc-macro

This crate is designed to work with the `rust-jsonrpc-macro` procedural macro:

```rust
use rust_jsonrpc_macro::jsonrpc_service;

jsonrpc_service!({
    service_name: MyService,
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS(["user"]) get_profile(()) -> UserProfile,
        WITH_PERMISSIONS(["admin"]) delete_user(UserId) -> (),
    ]
});

// Use with the generated builder
let service = MyServiceBuilder::new("/api")
    .auth_provider(JwtAuthProvider::new("secret"))
    .build();
```

## Example Auth Providers

### JWT Authentication
```rust
struct JwtAuthProvider { /* ... */ }
// Full JWT validation with claims extraction
```

### API Key Authentication
```rust
struct ApiKeyAuthProvider { /* ... */ }
// Simple API key lookup with rate limiting
```

### Composite Authentication
```rust
struct CompositeAuthProvider { /* ... */ }
// Try multiple auth methods in sequence
```

See the [`examples/`](../../examples/) directory for complete implementations.

## Re-exports

For convenience, this crate re-exports all types from `rust-jsonrpc-types`:

```rust
use rust_jsonrpc_core::{JsonRpcRequest, JsonRpcResponse, JsonRpcError};
```

## License

This project is licensed under the MIT License.