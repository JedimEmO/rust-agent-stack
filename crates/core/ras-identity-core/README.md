# ras-identity-core

Core traits and types for identity management in the Rust Agent Stack.

## Overview

This crate defines the foundational traits for identity providers and user permissions:

- `IdentityProvider` - Trait for verifying user identities
- `UserPermissions` - Trait for determining user permissions
- `VerifiedIdentity` - Represents a successfully verified identity

## Key Traits

### IdentityProvider

The main trait for identity verification:

```rust
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    async fn verify_identity(
        &self,
        provider_params: serde_json::Value,
    ) -> Result<VerifiedIdentity, IdentityError>;
    
    fn provider_name(&self) -> &str;
}
```

### UserPermissions

Trait for determining permissions based on identity:

```rust
#[async_trait]
pub trait UserPermissions: Send + Sync {
    async fn get_permissions(
        &self,
        identity: &VerifiedIdentity,
    ) -> Result<Vec<String>, PermissionError>;
}
```

## Key Types

### VerifiedIdentity

Represents a successfully verified user identity:

```rust
pub struct VerifiedIdentity {
    pub provider: String,
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
```

## Built-in Implementations

### NoopPermissions

Returns no permissions for any user (default):

```rust
let permissions = NoopPermissions;
```

### StaticPermissions

Returns the same permissions for all users:

```rust
let permissions = StaticPermissions::new(vec!["read".to_string(), "write".to_string()]);
```

## Usage with Provider Implementations

This crate is used by concrete identity provider implementations:
- `ras-identity-local` - Username/password authentication
- `ras-identity-oauth2` - OAuth2 authentication (Google, etc.)

## Example

```rust
use ras_identity_core::{IdentityProvider, VerifiedIdentity, IdentityError};
use async_trait::async_trait;

struct MyIdentityProvider;

#[async_trait]
impl IdentityProvider for MyIdentityProvider {
    async fn verify_identity(
        &self,
        provider_params: serde_json::Value,
    ) -> Result<VerifiedIdentity, IdentityError> {
        // Your identity verification logic here
        Ok(VerifiedIdentity {
            provider: "my-provider".to_string(),
            user_id: "user-123".to_string(),
            username: "john.doe".to_string(),
            email: Some("john@example.com".to_string()),
            metadata: None,
        })
    }
    
    fn provider_name(&self) -> &str {
        "my-provider"
    }
}
```

## Security Considerations

- Provider parameters use `serde_json::Value` to maintain decoupling
- Implementations should validate all inputs
- Sensitive data should not be stored in metadata fields
- Use constant-time comparisons for security-critical operations