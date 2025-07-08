# RAS Identity System Usage Guide

This guide provides everything you need to add authentication and authorization to your RAS stack application using the identity crates.

## Overview

The RAS identity system provides a flexible, secure authentication framework with:
- Multiple authentication providers (local users, OAuth2)
- JWT-based session management
- Fine-grained permission control
- Integration with JSON-RPC and REST services

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Client App    │────▶│ Identity Provider│────▶│ Session Service │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                           │
                                                           ▼
                        ┌─────────────────┐       ┌─────────────────┐
                        │ JSON-RPC/REST   │◀──────│ JwtAuthProvider │
                        │    Service      │       └─────────────────┘
                        └─────────────────┘
```

## Quick Start

### 1. Add Dependencies

```toml
[dependencies]
# Core authentication traits
ras-auth-core = { path = "../crates/core/ras-auth-core" }
ras-identity-core = { path = "../crates/core/ras-identity-core" }

# Session management (required)
ras-identity-session = { path = "../crates/identity/ras-identity-session" }

# Identity providers (choose what you need)
ras-identity-local = { path = "../crates/identity/ras-identity-local" }
ras-identity-oauth2 = { path = "../crates/identity/ras-identity-oauth2" }

# For JSON-RPC services
ras-jsonrpc-core = { path = "../crates/rpc/ras-jsonrpc-core" }
```

### 2. Basic Setup with Local Authentication

```rust
use ras_identity_session::{SessionService, SessionConfig, JwtAuthProvider};
use ras_identity_local::LocalUserProvider;
use ras_auth_core::AuthProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create session service with default config
    let session_service = SessionService::new(SessionConfig::default());
    
    // Create and configure local user provider
    let local_provider = LocalUserProvider::new();
    
    // Add some users
    local_provider.add_user(
        "admin",
        "secure_password123",
        Some("admin@example.com"),
        Some("Administrator")
    ).await?;
    
    // Register the provider with session service
    session_service.register_provider(Box::new(local_provider)).await;
    
    // Create JWT auth provider for your services
    let jwt_auth = JwtAuthProvider::new(Arc::new(session_service));
    
    // Now use jwt_auth with your JSON-RPC or REST services
    Ok(())
}
```

## Identity Providers

### Local User Provider

The local user provider handles username/password authentication with secure password hashing:

```rust
use ras_identity_local::LocalUserProvider;
use serde_json::json;

// Create provider
let provider = LocalUserProvider::new();

// Add users
provider.add_user("alice", "password123", 
    Some("alice@example.com"), 
    Some("Alice Smith")
).await?;

// Authenticate
let auth_payload = json!({
    "username": "alice",
    "password": "password123"
});

let identity = provider.verify(auth_payload).await?;
println!("Authenticated: {}", identity.display_name.unwrap_or_default());
```

**Security Features:**
- Argon2 password hashing
- Timing attack resistance
- Username enumeration prevention
- Rate limiting (5 concurrent attempts)

### OAuth2 Provider

The OAuth2 provider supports external authentication providers like Google:

```rust
use ras_identity_oauth2::{OAuth2Provider, OAuth2Config, ProviderConfig};
use oauth2::{ClientId, ClientSecret, AuthUrl, TokenUrl};

// Configure OAuth2 provider
let google_config = ProviderConfig {
    client_id: ClientId::new("your-client-id".to_string()),
    client_secret: ClientSecret::new("your-client-secret".to_string()),
    auth_url: AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
    token_url: TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?,
    user_info_url: "https://www.googleapis.com/oauth2/v2/userinfo".to_string(),
    redirect_url: "http://localhost:3000/auth/callback".to_string(),
    scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
    user_info_mapping: Default::default(), // Uses standard mapping
};

let config = OAuth2Config {
    providers: vec![("google".to_string(), google_config)].into_iter().collect(),
};

let oauth_provider = OAuth2Provider::new(config);
```

**OAuth2 Flow:**

1. **Start Authorization:**
```rust
let start_payload = json!({
    "type": "StartFlow",
    "provider_id": "google"
});

match provider.verify(start_payload).await {
    Err(IdentityError::OAuth2(OAuth2Response::AuthorizationUrl { url, state })) => {
        // Redirect user to authorization URL
        println!("Redirect to: {}", url);
        // Store state for CSRF protection
    }
    _ => panic!("Unexpected response"),
}
```

2. **Handle Callback:**
```rust
let callback_payload = json!({
    "type": "Callback",
    "provider_id": "google",
    "code": "authorization_code_from_provider",
    "state": "stored_csrf_state"
});

let identity = provider.verify(callback_payload).await?;
```

## Session Management

The `SessionService` orchestrates the complete authentication flow:

```rust
use ras_identity_session::{SessionService, SessionConfig};
use std::time::Duration;

// Configure session service
let config = SessionConfig {
    jwt_secret: "your-secret-key".to_string(),
    jwt_ttl: Duration::from_secs(3600), // 1 hour
    jwt_algorithm: "HS256".to_string(),
    refresh_enabled: false,
    refresh_ttl: None,
};

let session_service = SessionService::new(config);

// Register multiple providers
session_service.register_provider(Box::new(local_provider)).await;
session_service.register_provider(Box::new(oauth_provider)).await;
```

### Creating Sessions

```rust
// Authenticate and create session
let auth_payload = json!({
    "username": "alice",
    "password": "password123"
});

let jwt_token = session_service.begin_session("local", auth_payload).await?;
println!("JWT Token: {}", jwt_token);

// Verify session
let authenticated_user = session_service.verify_session(&jwt_token).await?;
println!("User ID: {}", authenticated_user.user_id);
println!("Permissions: {:?}", authenticated_user.permissions);

// End session (logout)
session_service.end_session(&jwt_token).await?;
```

## Permission Management

Implement custom permission logic using the `UserPermissions` trait:

```rust
use ras_identity_core::{UserPermissions, VerifiedIdentity};
use async_trait::async_trait;

struct RoleBasedPermissions {
    // Your permission logic
}

#[async_trait]
impl UserPermissions for RoleBasedPermissions {
    async fn get_permissions(&self, identity: &VerifiedIdentity) -> Vec<String> {
        // Example: Grant permissions based on email domain
        match &identity.email {
            Some(email) if email.ends_with("@admin.com") => {
                vec!["admin".to_string(), "user".to_string()]
            }
            Some(_) => vec!["user".to_string()],
            None => vec![],
        }
    }
}

// Use with session service
let permissions = Arc::new(RoleBasedPermissions {});
session_service.with_permissions(permissions);
```

## Integration with Services

### JSON-RPC Service Integration

```rust
use ras_jsonrpc_macro::jsonrpc_service;
use ras_identity_session::JwtAuthProvider;

// Define your service with authentication
jsonrpc_service!({
    service_name: MyApiService,
    methods: [
        // Public method
        UNAUTHORIZED get_status(()) -> Status,
        
        // Requires authentication
        AUTHENTICATED get_profile(()) -> UserProfile,
        
        // Requires specific permissions
        WITH_PERMISSIONS(["admin"]) delete_user(DeleteUserRequest) -> (),
    ]
});

// Implement service
struct MyApiServiceImpl;

#[async_trait]
impl MyApiService for MyApiServiceImpl {
    async fn get_status(&self) -> Result<Status, Error> {
        Ok(Status { healthy: true })
    }
    
    async fn get_profile(&self, user: AuthenticatedUser) -> Result<UserProfile, Error> {
        // Access user.user_id, user.permissions, etc.
        Ok(UserProfile { 
            id: user.user_id,
            permissions: user.permissions.into_iter().collect(),
        })
    }
    
    async fn delete_user(&self, _user: AuthenticatedUser, req: DeleteUserRequest) -> Result<(), Error> {
        // Only users with "admin" permission can reach here
        Ok(())
    }
}

// Set up with Axum
use axum::Router;

let jwt_auth = JwtAuthProvider::new(Arc::new(session_service));
let service = MyApiServiceImpl;

let app = Router::new()
    .nest("/api", 
        MyApiServiceBuilder::new(service)
            .with_auth_provider(Arc::new(jwt_auth))
            .build()
    );
```

### REST Service Integration

```rust
use ras_rest_macro::rest_service;

rest_service!({
    service_name: UserApi,
    base_path: "/api/v1",
    endpoints: [
        // Public endpoint
        UNAUTHORIZED GET "/health" health_check() -> HealthResponse,
        
        // Authenticated endpoint
        AUTHENTICATED GET "/me" get_current_user() -> UserResponse,
        
        // Permission-protected endpoint
        WITH_PERMISSIONS(["admin"]) DELETE "/users/:id" delete_user(PathParam<String>) -> (),
    ]
});
```

## Complete Example

Here's a complete example showing a typical setup:

```rust
use ras_identity_session::{SessionService, SessionConfig, JwtAuthProvider};
use ras_identity_local::LocalUserProvider;
use ras_identity_oauth2::{OAuth2Provider, OAuth2Config, ProviderConfig};
use ras_jsonrpc_macro::jsonrpc_service;
use axum::{Router, routing::get};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

// Define your API
jsonrpc_service!({
    service_name: TodoService,
    methods: [
        UNAUTHORIZED health_check(()) -> HealthStatus,
        AUTHENTICATED list_todos(()) -> Vec<Todo>,
        WITH_PERMISSIONS(["user"]) create_todo(CreateTodoRequest) -> Todo,
        WITH_PERMISSIONS(["admin"]) delete_all_todos(()) -> (),
    ]
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Set up session service
    let session_config = SessionConfig {
        jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string()),
        jwt_ttl: std::time::Duration::from_secs(3600),
        jwt_algorithm: "HS256".to_string(),
        refresh_enabled: false,
        refresh_ttl: None,
    };
    
    let session_service = Arc::new(SessionService::new(session_config));
    
    // 2. Set up local authentication
    let local_provider = LocalUserProvider::new();
    local_provider.add_user("user", "password", Some("user@example.com"), Some("User")).await?;
    local_provider.add_user("admin", "admin123", Some("admin@example.com"), Some("Admin")).await?;
    
    session_service.register_provider(Box::new(local_provider)).await;
    
    // 3. Set up permissions
    use ras_identity_core::{UserPermissions, VerifiedIdentity};
    use async_trait::async_trait;
    
    struct SimplePermissions;
    
    #[async_trait]
    impl UserPermissions for SimplePermissions {
        async fn get_permissions(&self, identity: &VerifiedIdentity) -> Vec<String> {
            match identity.subject.as_str() {
                "admin" => vec!["user".to_string(), "admin".to_string()],
                _ => vec!["user".to_string()],
            }
        }
    }
    
    session_service.with_permissions(Arc::new(SimplePermissions));
    
    // 4. Create authentication endpoints
    let auth_router = Router::new()
        .route("/login", get(login_handler))
        .route("/logout", get(logout_handler));
    
    // 5. Create API with authentication
    let jwt_auth = JwtAuthProvider::new(session_service.clone());
    let todo_service = TodoServiceImpl::new();
    
    let api_router = TodoServiceBuilder::new(todo_service)
        .with_auth_provider(Arc::new(jwt_auth))
        .build();
    
    // 6. Combine everything
    let app = Router::new()
        .nest("/auth", auth_router)
        .nest("/api", api_router)
        .layer(CorsLayer::permissive())
        .with_state(session_service);
    
    // 7. Start server
    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

## Best Practices

### 1. Security

- **Use strong JWT secrets**: Generate cryptographically secure secrets
- **Set appropriate TTLs**: Balance security and user experience
- **Enable HTTPS**: Always use TLS in production
- **Validate permissions**: Check permissions at the service level
- **Handle errors gracefully**: Don't leak information in error messages

### 2. Configuration

```rust
// Use environment variables for sensitive config
let config = SessionConfig {
    jwt_secret: std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set"),
    jwt_ttl: Duration::from_secs(
        std::env::var("JWT_TTL_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()?
    ),
    // ...
};
```

### 3. Error Handling

```rust
use ras_identity_core::IdentityError;

match session_service.begin_session("local", payload).await {
    Ok(token) => {
        // Success
    }
    Err(IdentityError::InvalidCredentials) => {
        // Wrong username/password
    }
    Err(IdentityError::ProviderNotFound(_)) => {
        // Provider not registered
    }
    Err(e) => {
        // Other errors
    }
}
```

### 4. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_authentication_flow() {
        // Set up test providers
        let provider = LocalUserProvider::new();
        provider.add_user("test", "test123", None, None).await.unwrap();
        
        let session_service = SessionService::new(SessionConfig::default());
        session_service.register_provider(Box::new(provider)).await;
        
        // Test authentication
        let token = session_service.begin_session("local", json!({
            "username": "test",
            "password": "test123"
        })).await.unwrap();
        
        // Verify token
        let user = session_service.verify_session(&token).await.unwrap();
        assert_eq!(user.user_id, "test");
    }
}
```

## Troubleshooting

### Common Issues

1. **"Provider not found" error**
   - Ensure you've registered the provider with `session_service.register_provider()`
   - Check the provider ID matches (e.g., "local" for LocalUserProvider)

2. **JWT validation failures**
   - Verify the JWT secret is consistent across services
   - Check token hasn't expired (default TTL is 1 hour)
   - Ensure the token is passed in the correct format

3. **Permission denied errors**
   - Verify your `UserPermissions` implementation returns expected permissions
   - Check the method annotation matches required permissions
   - Use `AUTHENTICATED` for methods that only need login, not specific permissions

4. **OAuth2 redirect issues**
   - Ensure redirect URLs are correctly configured in provider settings
   - Check CORS settings allow the callback domain
   - Verify state parameter is preserved through the flow

## Advanced Topics

### Custom Identity Providers

Implement the `IdentityProvider` trait for custom authentication:

```rust
use ras_identity_core::{IdentityProvider, VerifiedIdentity, IdentityError};
use async_trait::async_trait;

struct LdapProvider {
    // LDAP configuration
}

#[async_trait]
impl IdentityProvider for LdapProvider {
    fn id(&self) -> &str {
        "ldap"
    }
    
    async fn verify(&self, payload: serde_json::Value) -> Result<VerifiedIdentity, IdentityError> {
        // Implement LDAP authentication
        todo!()
    }
}
```

### Session Revocation

Implement immediate session revocation:

```rust
// End specific session
session_service.end_session(&jwt_token).await?;

// End all sessions for a user
// (Requires custom implementation tracking user->session mapping)
```

### Refresh Tokens

Enable refresh tokens for long-lived sessions:

```rust
let config = SessionConfig {
    refresh_enabled: true,
    refresh_ttl: Some(Duration::from_days(30)),
    // ...
};

// Use refresh token to get new access token
let new_token = session_service.refresh_session(&refresh_token).await?;
```

## Conclusion

The RAS identity system provides a robust foundation for authentication in your applications. Start with basic local authentication and progressively add OAuth2 providers and custom permission logic as needed. The modular design ensures you can adapt the system to your specific requirements while maintaining security best practices.

For more examples, check out:
- `/examples/google-oauth-example/` - Complete OAuth2 integration
- `/examples/basic-jsonrpc-service/` - JSON-RPC with authentication
- `/examples/bidirectional-chat/` - WebSocket authentication