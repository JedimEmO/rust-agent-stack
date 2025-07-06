# ras-auth-core

Core authentication traits and types for the Rust Agent Stack authentication system.

## Overview

This crate provides the foundational traits and types used across all RAS authentication implementations:

- `AuthProvider` - Main trait for authentication providers
- `AuthenticatedUser` - Represents an authenticated user with permissions
- `AuthError` - Common error types for authentication failures

## Key Types

### AuthProvider

The main trait that authentication providers must implement:

```rust
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, token: &str) -> Result<AuthenticatedUser, AuthError>;
}
```

### AuthenticatedUser

Represents a successfully authenticated user:

```rust
pub struct AuthenticatedUser {
    pub id: String,
    pub username: String,
    pub permissions: Vec<String>,
}
```

### AuthError

Common authentication error types:

```rust
pub enum AuthError {
    InvalidToken,
    ExpiredToken,
    InvalidCredentials,
    InsufficientPermissions,
    InternalError(String),
}
```

## Usage

This crate is typically used as a dependency by:
- Authentication provider implementations (JWT, OAuth, etc.)
- JSON-RPC and REST service macros
- Service implementations requiring authentication

## Example

```rust
use ras_auth_core::{AuthProvider, AuthenticatedUser, AuthError};
use async_trait::async_trait;

struct MyAuthProvider;

#[async_trait]
impl AuthProvider for MyAuthProvider {
    async fn authenticate(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        // Your authentication logic here
        Ok(AuthenticatedUser {
            id: "user-123".to_string(),
            username: "john.doe".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
        })
    }
}
```

## Integration

This crate integrates seamlessly with:
- `ras-jsonrpc-macro` - For JSON-RPC service authentication
- `ras-rest-macro` - For REST API authentication
- `ras-identity-session` - For JWT-based authentication
- `ras-jsonrpc-bidirectional-server` - For WebSocket authentication