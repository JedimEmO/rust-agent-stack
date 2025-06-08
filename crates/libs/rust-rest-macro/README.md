# rust-rest-macro

A procedural macro for creating type-safe REST APIs with authentication integration and OpenAPI document generation.

## Features

- **Type-safe REST endpoints**: Generate axum-based REST services from macro definitions
- **Authentication integration**: Seamless integration with `rust-jsonrpc-core::AuthProvider`
- **Permission-based access control**: Support for role-based authorization
- **OpenAPI 3.0 generation**: Automatic OpenAPI documentation using schemars
- **HTTP methods**: Support for GET, POST, PUT, DELETE, PATCH
- **Path parameters**: Type-safe path parameter extraction
- **Request/Response bodies**: JSON request and response handling

## Usage

### Basic Example

```rust
use rust_rest_macro::rest_service;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, JsonSchema)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct CreateUserRequest {
    name: String,
    email: String,
}

rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,
    endpoints: [
        GET UNAUTHORIZED users() -> Vec<User>,
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> User,
        GET WITH_PERMISSIONS(["user"]) users/{id: i32}() -> User,
        PUT WITH_PERMISSIONS(["admin"]) users/{id: i32}(CreateUserRequest) -> User,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: i32}() -> (),
    ]
});

// The macro generates:
// - UserServiceTrait: A trait with async methods for each endpoint
// - UserServiceBuilder: A builder for configuring handlers and auth providers
```

### Service Configuration

```rust
let service = UserServiceBuilder::new()
    .auth_provider(my_auth_provider)
    .get_users_handler(|_| async { 
        // Return list of users
        Ok(vec![])
    })
    .post_users_handler(|user, request| async {
        // Create new user with admin permissions
        Ok(User { id: 1, name: request.name, email: request.email })
    })
    .get_users_by_id_handler(|user, id| async {
        // Get user by ID with user permissions
        Ok(User { id, name: "John".to_string(), email: "john@example.com".to_string() })
    })
    .build();

// Use with axum
let app = axum::Router::new().merge(service);
```

### OpenAPI Generation

```rust
// Generate OpenAPI document programmatically
let openapi_doc = generate_userservice_openapi();

// Write to file
generate_userservice_openapi_to_file().unwrap();
```

## Macro Syntax

### Service Definition

```rust
rest_service!({
    service_name: ServiceName,           // Name for the generated trait and builder
    base_path: "/api/v1",               // Base path for all endpoints
    openapi: true,                      // Enable OpenAPI generation (optional)
    // or: openapi: { output: "path/to/spec.json" },
    endpoints: [
        // Endpoint definitions...
    ]
});
```

### Endpoint Definition

```rust
METHOD AUTH_REQUIREMENT path(RequestType) -> ResponseType,
```

- **METHOD**: `GET`, `POST`, `PUT`, `DELETE`, or `PATCH`
- **AUTH_REQUIREMENT**: 
  - `UNAUTHORIZED`: No authentication required
  - `WITH_PERMISSIONS(["perm1", "perm2"])`: Requires authentication and specified permissions
- **path**: URL path with optional parameters in `{param: Type}` format
- **RequestType**: Optional request body type (omit `()` for no body)
- **ResponseType**: Response type

### Examples

```rust
// Simple GET endpoint with no auth
GET UNAUTHORIZED users() -> Vec<User>,

// POST endpoint requiring admin permission with request body
POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> User,

// GET endpoint with path parameter requiring user permission
GET WITH_PERMISSIONS(["user"]) users/{id: i32}() -> User,

// Multiple path parameters
GET UNAUTHORIZED posts/{user_id: i32}/comments/{comment_id: String}() -> Comment,
```

## Authentication Integration

The macro integrates with `rust-jsonrpc-core::AuthProvider` for authentication:

```rust
use rust_jsonrpc_core::AuthProvider;

struct MyAuthProvider;

#[async_trait::async_trait]
impl AuthProvider for MyAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        // Validate JWT token and return authenticated user
    }
}

let service = UserServiceBuilder::new()
    .auth_provider(MyAuthProvider)
    .build();
```

## Requirements

All request and response types must implement:
- `serde::Serialize` + `serde::Deserialize`
- `schemars::JsonSchema` (for OpenAPI generation)

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
struct MyType {
    field: String,
}
```

## Generated Code

The macro generates:
1. **Service Trait**: `{ServiceName}Trait` with async methods for each endpoint
2. **Builder**: `{ServiceName}Builder` for configuration
3. **OpenAPI Functions**: `generate_{servicename}_openapi()` and `generate_{servicename}_openapi_to_file()`

## Integration with Axum

The generated service returns an `axum::Router` that can be used directly or merged with other routers:

```rust
let app = axum::Router::new()
    .merge(user_service)
    .merge(other_service);
```

## OpenAPI 3.0 Support

- Automatic schema generation from Rust types using schemars
- Authentication requirements in OpenAPI security schemes
- Permission metadata as OpenAPI extensions
- Path parameters and request/response schemas
- Standard HTTP error responses