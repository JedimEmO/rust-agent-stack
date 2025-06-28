# ras-identity-session

JWT-based session management for the Rust Agent Stack authentication system.

## Overview

This crate provides session management capabilities using JSON Web Tokens (JWT):
- JWT creation and validation
- Session tracking and revocation
- Integration with RAS authentication system
- Configurable token TTL and algorithms

## Features

- **JWT Sessions**: Create and validate JWT tokens with custom claims
- **Session Registry**: Track active sessions for revocation support
- **AuthProvider Implementation**: `JwtAuthProvider` for seamless integration
- **Flexible Configuration**: Configurable secrets, TTL, and algorithms
- **Permission Embedding**: User permissions stored in JWT claims

## Usage

### Basic Session Service Setup

```rust
use ras_identity_session::{SessionService, SessionConfig};
use ras_identity_core::{VerifiedIdentity, StaticPermissions};

// Create session service with default config
let session_service = SessionService::new(
    "your-secret-key".to_string(),
    3600, // 1 hour TTL
    StaticPermissions::new(vec!["read".to_string()]),
);

// Or with custom config
let config = SessionConfig {
    secret: "your-secret-key".to_string(),
    ttl_seconds: 7200, // 2 hours
    algorithm: "HS256".to_string(),
    refresh_enabled: false,
    refresh_ttl_seconds: None,
};
let session_service = SessionService::with_config(config, permissions);
```

### Creating Sessions

```rust
// After verifying identity with an IdentityProvider
let verified_identity = VerifiedIdentity {
    provider: "local".to_string(),
    user_id: "user-123".to_string(),
    username: "alice".to_string(),
    email: Some("alice@example.com".to_string()),
    metadata: None,
};

// Create JWT session
let jwt_token = session_service.create_session(verified_identity).await?;
```

### Using JwtAuthProvider

```rust
use ras_identity_session::JwtAuthProvider;
use ras_auth_core::AuthProvider;

// Create auth provider from session service
let auth_provider = JwtAuthProvider::from_session_service(session_service);

// Authenticate tokens
let user = auth_provider.authenticate(&jwt_token).await?;
println!("Authenticated user: {} with permissions: {:?}", 
    user.username, user.permissions);
```

### Session Management

```rust
// Get session info
if let Some(info) = session_service.get_session(&jti).await {
    println!("Session for user: {}", info.user_id);
}

// End a session (revoke)
session_service.end_session(&jti).await?;

// Check active sessions
let active_count = session_service.active_session_count().await;
```

## JWT Structure

The generated JWTs include:
- **Standard Claims**: `iss`, `sub`, `exp`, `iat`, `jti`
- **Custom Claims**: 
  - `username`: User's display name
  - `permissions`: Array of permission strings
  - `provider`: Identity provider name

Example JWT payload:
```json
{
  "iss": "ras",
  "sub": "user-123",
  "exp": 1700000000,
  "iat": 1699996400,
  "jti": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "permissions": ["read", "write"],
  "provider": "local"
}
```

## Integration Examples

### With JSON-RPC Services

```rust
use ras_jsonrpc_macro::jsonrpc_service;

jsonrpc_service!({
    service_name: MyService,
    auth_provider: JwtAuthProvider,
    methods: [
        WITH_PERMISSIONS(["read"]) get_data(GetRequest) -> GetResponse,
        WITH_PERMISSIONS(["write"]) update_data(UpdateRequest) -> UpdateResponse,
    ]
});
```

### With WebSocket Authentication

The session service integrates with bidirectional JSON-RPC WebSocket services, supporting multiple authentication header formats:
- `Authorization: Bearer <token>`
- `X-Auth-Token: <token>`
- `Sec-WebSocket-Protocol: token.<token>`

## Security Considerations

1. **Secret Management**
   - Use strong, randomly generated secrets
   - Store secrets securely (environment variables, secret management systems)
   - Rotate secrets periodically

2. **Token Expiration**
   - Set appropriate TTL based on security requirements
   - Consider implementing refresh tokens for long-lived sessions

3. **Session Revocation**
   - Track active sessions for immediate revocation capability
   - Clean up expired sessions periodically

4. **HTTPS Only**
   - Always transmit JWTs over HTTPS in production
   - Set secure cookie flags when using cookies

## Configuration Options

- **Secret**: JWT signing secret (required)
- **TTL**: Token time-to-live in seconds
- **Algorithm**: JWT signing algorithm (default: HS256)
- **Refresh**: Enable/disable refresh token support (experimental)