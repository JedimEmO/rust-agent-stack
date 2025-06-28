# ras-identity-local

Local username/password identity provider for the Rust Agent Stack.

## Overview

This crate provides a secure local authentication implementation using:
- Argon2 password hashing (industry standard)
- Protection against timing attacks
- Prevention of username enumeration
- Thread-safe concurrent request handling

## Features

- **Secure Password Storage**: Uses Argon2id for password hashing
- **Attack Protection**: Constant-time operations prevent timing attacks
- **Rate Limiting**: Built-in semaphore limits concurrent authentication attempts
- **Thread-Safe**: Safe for use in async multi-threaded environments

## Usage

### Basic Setup

```rust
use ras_identity_local::{LocalUserProvider, LocalUserStore};
use ras_identity_core::IdentityProvider;

// Create a user store
let mut store = LocalUserStore::new();
store.add_user("alice", "secure_password").await?;
store.add_user("bob", "another_password").await?;

// Create the provider
let provider = LocalUserProvider::new(store);

// Verify identity
let params = serde_json::json!({
    "username": "alice",
    "password": "secure_password"
});

let identity = provider.verify_identity(params).await?;
assert_eq!(identity.username, "alice");
```

### Integration with Session Service

```rust
use ras_identity_local::LocalUserProvider;
use ras_identity_session::SessionService;
use ras_identity_core::StaticPermissions;

// Set up identity provider
let provider = LocalUserProvider::new(store);

// Set up session service with permissions
let permissions = StaticPermissions::new(vec!["read".to_string(), "write".to_string()]);
let session_service = SessionService::new(
    "your-secret-key".to_string(),
    3600, // 1 hour TTL
    permissions,
);

// Complete authentication flow
let identity = provider.verify_identity(login_params).await?;
let jwt = session_service.create_session(identity).await?;
```

## Security Features

### Attack Protection

1. **Timing Attack Resistance**
   - Uses dummy Argon2 hash for non-existent users
   - Ensures consistent response times regardless of user existence

2. **Username Enumeration Prevention**
   - Always returns `InvalidCredentials` for any failure
   - No distinction between "user not found" and "wrong password"

3. **Rate Limiting**
   - Semaphore limits to 5 concurrent authentication attempts
   - Prevents brute force attacks

4. **Input Validation**
   - Handles empty credentials gracefully
   - Validates input format and length
   - Safe handling of special characters

### Password Requirements

The implementation uses Argon2 default settings:
- Memory cost: 19 MiB
- Time cost: 2 iterations
- Parallelism: 1 thread
- Output length: 32 bytes

## Testing

The crate includes comprehensive security tests:
- Username enumeration attempts
- Timing attack detection
- Concurrent request handling
- Password spraying simulation
- Input validation edge cases

Run tests with:
```bash
cargo test -p ras-identity-local
```

## Best Practices

1. **Never log passwords** or password hashes
2. **Use strong passwords** - consider implementing password policies
3. **Monitor failed attempts** - implement account lockout in production
4. **Rotate secrets** - change JWT secrets periodically
5. **Use HTTPS** - always use TLS in production environments