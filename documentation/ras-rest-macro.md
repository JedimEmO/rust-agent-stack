# RAS REST Macro Documentation

The `ras-rest-macro` crate provides a powerful procedural macro for building type-safe REST APIs in Rust with automatic client generation for both native Rust and TypeScript environments.

## Table of Contents

1. [Overview](#overview)
2. [Installation](#installation)
3. [Basic Usage](#basic-usage)
4. [Macro Syntax](#macro-syntax)
5. [Authentication & Authorization](#authentication--authorization)
6. [Generated Code](#generated-code)
7. [TypeScript Client Usage](#typescript-client-usage)
8. [OpenAPI Documentation](#openapi-documentation)
9. [Error Handling](#error-handling)
10. [Advanced Features](#advanced-features)
11. [Complete Example](#complete-example)

## Overview

The `rest_service!` macro generates:
- A service trait for implementing your REST API
- An Axum router builder with authentication support
- Native Rust client with async/await support
- OpenAPI 3.0 specification for TypeScript client generation
- Built-in Swagger UI hosting (optional)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ras-rest-macro = "0.1.0"
ras-rest-core = "0.1.0"
ras-auth-core = "0.1.0"  # For authentication
serde = { version = "1.0", features = ["derive"] }
schemars = "0.8"  # Required for OpenAPI generation
axum = "0.7"  # Web framework
tokio = { version = "1", features = ["full"] }

[features]
server = []  # Enable server-side code generation
client = []  # Enable native client generation
```

## Basic Usage

### 1. Define Your API Types

All request and response types must implement `Serialize`, `Deserialize`, and `JsonSchema`:

```rust
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UsersResponse {
    pub users: Vec<User>,
    pub total: usize,
}
```

### 2. Define Your REST Service

```rust
use ras_rest_macro::rest_service;

rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,
    docs_path: "/docs",
    endpoints: [
        // Public endpoints (no auth required)
        GET UNAUTHORIZED users() -> UsersResponse,
        GET UNAUTHORIZED users/{id: String}() -> User,
        
        // Protected endpoints (auth required)
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> User,
        PUT WITH_PERMISSIONS(["admin"]) users/{id: String}(UpdateUserRequest) -> User,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: String}() -> (),
    ]
});
```

### 3. Implement the Generated Trait

```rust
use ras_auth_core::AuthenticatedUser;
use ras_rest_core::{RestResult, RestResponse, RestError};

struct UserServiceImpl {
    // Your service state
}

#[async_trait::async_trait]
impl UserServiceTrait for UserServiceImpl {
    async fn get_users(&self) -> RestResult<UsersResponse> {
        // Implementation
        Ok(RestResponse::ok(UsersResponse {
            users: vec![],
            total: 0,
        }))
    }

    async fn get_users_by_id(&self, id: String) -> RestResult<User> {
        // Implementation
        users.get(&id)
            .cloned()
            .map(|user| RestResponse::ok(user))
            .ok_or_else(|| RestError::not_found("User not found"))
    }

    async fn post_users(
        &self,
        user: &AuthenticatedUser,  // Auto-injected for authenticated endpoints
        request: CreateUserRequest,
    ) -> RestResult<User> {
        // Implementation with access to authenticated user
        Ok(RestResponse::created(new_user))
    }
    
    // ... implement other methods
}
```

### 4. Create and Run the Server

```rust
use axum::Router;

#[tokio::main]
async fn main() {
    let service = UserServiceImpl { /* ... */ };
    let auth_provider = MyAuthProvider { /* ... */ };

    let api_router = UserServiceBuilder::new(service)
        .auth_provider(auth_provider)
        .with_usage_tracker(|headers, user, method, path| async move {
            // Log API usage
        })
        .with_method_duration_tracker(|method, path, user, duration| async move {
            // Track performance metrics
        })
        .build();

    let app = Router::new().merge(api_router);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
}
```

## Macro Syntax

### Full Syntax

```rust
rest_service!({
    service_name: ServiceName,           // Required: Name for generated types
    base_path: "/api/v1",               // Required: Base URL path
    openapi: true,                      // Optional: Enable OpenAPI generation
    openapi: { output: "api.json" },    // Optional: Custom OpenAPI output path
    serve_docs: true,                   // Optional: Enable Swagger UI
    docs_path: "/docs",                 // Optional: Swagger UI path (default: "/docs")
    ui_theme: "dark",                   // Optional: Swagger UI theme
    endpoints: [
        // Endpoint definitions
    ]
});
```

### Endpoint Syntax

```
METHOD AUTH_REQUIREMENT path/{param: Type}/segments(RequestType) -> ResponseType
```

- **METHOD**: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
- **AUTH_REQUIREMENT**: 
  - `UNAUTHORIZED` - No authentication required
  - `WITH_PERMISSIONS(["permission1", "permission2"])` - Requires all listed permissions (AND)
  - `WITH_PERMISSIONS(["perm1"] | ["perm2"])` - Requires any permission group (OR)
- **Path**: URL path with optional parameters in `{name: Type}` format
- **RequestType**: Optional request body type (omit for GET/DELETE)
- **ResponseType**: Response body type (use `()` for empty responses)

### Path Parameters

Path parameters are defined inline using `{name: Type}` syntax:

```rust
GET UNAUTHORIZED users/{id: String}() -> User,
PUT WITH_PERMISSIONS(["admin"]) posts/{post_id: i32}/comments/{comment_id: i32}(UpdateCommentRequest) -> Comment,
```

## Authentication & Authorization

### Setting Up Authentication

The macro integrates with `ras-auth-core` for authentication:

```rust
use ras_auth_core::{AuthProvider, AuthenticatedUser, AuthResult};

struct MyAuthProvider;

#[async_trait::async_trait]
impl AuthProvider for MyAuthProvider {
    async fn authenticate(&self, token: String) -> AuthResult<AuthenticatedUser> {
        // Validate JWT token and return user info
    }

    async fn check_permissions(
        &self,
        user: &AuthenticatedUser,
        required_permissions: &[String],
    ) -> AuthResult<()> {
        // Check if user has required permissions
    }
}
```

### Permission Groups

Use OR logic between permission groups and AND logic within groups:

```rust
// Requires either admin OR (moderator AND editor)
WITH_PERMISSIONS(["admin"] | ["moderator", "editor"])
```

## Generated Code

The macro generates several components:

### 1. Service Trait

```rust
#[async_trait::async_trait]
pub trait UserServiceTrait: Send + Sync + 'static {
    async fn get_users(&self) -> RestResult<UsersResponse>;
    async fn get_users_by_id(&self, id: String) -> RestResult<User>;
    async fn post_users(&self, user: &AuthenticatedUser, request: CreateUserRequest) -> RestResult<User>;
    // ... other methods
}
```

### 2. Service Builder

```rust
pub struct UserServiceBuilder<T: UserServiceTrait> {
    // ...
}

impl<T: UserServiceTrait> UserServiceBuilder<T> {
    pub fn new(service: T) -> Self;
    pub fn auth_provider<A: AuthProvider>(self, provider: A) -> Self;
    pub fn with_usage_tracker<F, Fut>(self, tracker: F) -> Self;
    pub fn with_method_duration_tracker<F, Fut>(self, tracker: F) -> Self;
    pub fn build(self) -> axum::Router;
}
```

### 3. Native Rust Client

```rust
pub struct UserServiceClient {
    // ...
}

impl UserServiceClient {
    pub fn builder(server_url: impl Into<String>) -> UserServiceClientBuilder;
    pub fn set_bearer_token(&mut self, token: Option<impl Into<String>>);
    
    // Generated methods matching endpoints
    pub async fn get_users(&self) -> Result<UsersResponse, Box<dyn Error>>;
    pub async fn get_users_by_id(&self, id: String) -> Result<User, Box<dyn Error>>;
    pub async fn post_users(&self, body: CreateUserRequest) -> Result<User, Box<dyn Error>>;
    
    // Methods with custom timeout
    pub async fn get_users_with_timeout(&self, timeout: Option<Duration>) -> Result<UsersResponse, Box<dyn Error>>;
}
```

### 4. OpenAPI Generation

The macro generates an OpenAPI 3.0 specification that can be used to generate TypeScript clients:

```rust
// Generated function to create OpenAPI spec
pub fn generate_userservice_openapi() -> String {
    // Returns OpenAPI 3.0 JSON specification
}

// Generated function to write OpenAPI spec to file
pub fn generate_userservice_openapi_to_file() -> std::io::Result<()> {
    // Writes to target/openapi/userservice.json
}
```

## TypeScript Client Usage

### 1. Generate OpenAPI Specification

Add a `build.rs` file to your backend crate to generate the OpenAPI spec at compile time:

```rust
// backend/build.rs
fn main() {
    // Import your API module
    use rest_api;
    
    // Generate OpenAPI spec to target directory
    rest_api::generate_userservice_openapi_to_file()
        .expect("Failed to generate OpenAPI spec");
}
```

This creates `target/openapi/userservice.json` during compilation.

### 2. Set Up TypeScript Client Generation

Install dependencies:

```bash
cd typescript-example
npm install @hey-api/openapi-ts @hey-api/client-fetch --save-dev
```

Create `openapi-ts.config.ts`:

```typescript
import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  client: '@hey-api/client-fetch',
  input: '../backend/target/openapi/userservice.json',
  output: {
    path: './src/generated',
    format: 'prettier',
    lint: 'eslint',
  },
});
```

Update your `package.json`:

```json
{
  "scripts": {
    "generate": "openapi-ts",
    "dev": "npm run generate && vite",
    "build": "npm run generate && vite build"
  }
}
```

### 3. TypeScript Usage

```typescript
import * as api from './generated/services.gen';
import type { User, CreateUserRequest, UsersResponse } from './generated/types.gen';

// Configuration object for all requests
const config = {
  baseUrl: 'http://localhost:3000/api/v1',
  headers: {
    Authorization: 'Bearer jwt-token'
  }
};

// Make API calls with named methods
const response = await api.getUsers(config);
if (response.data) {
  const users = response.data.users;
}

// GET with path parameter
const userResponse = await api.getUsersId({
  ...config,
  path: { id: '123' }
});

// POST with typed body
const newUser: CreateUserRequest = {
  name: 'John Doe',
  email: 'john@example.com'
};

const created = await api.postUsers({
  ...config,
  body: newUser
});

// DELETE request
await api.deleteUsersId({
  ...config,
  path: { id: '123' }
});
```

### 4. Benefits Over WASM

- **Smaller Bundle Size**: ~10KB vs ~200KB+ for WASM
- **Better Developer Experience**: Standard TypeScript/JavaScript
- **Universal Compatibility**: Works in Node.js, Deno, Bun, and browsers
- **Better Tree-shaking**: Standard JavaScript optimization applies
- **Easier Debugging**: Standard network requests in DevTools

## OpenAPI Documentation

### Enabling OpenAPI Generation

```rust
rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,                    // Generate to target/openapi/UserService.json
    openapi: { output: "api.json" },  // Custom output path
    serve_docs: true,                 // Enable Swagger UI
    docs_path: "/docs",               // Swagger UI path
    // ...
});
```

### Generated OpenAPI Features

- Full endpoint documentation with request/response schemas
- Authentication requirements via `x-authentication` extension
- Permission requirements via `x-permissions` extension
- JSON Schema generation for all types
- Swagger UI integration

### Accessing OpenAPI Documentation

1. **Swagger UI**: Navigate to `http://localhost:3000/api/v1/docs`
2. **OpenAPI JSON**: Available at `http://localhost:3000/api/v1/openapi.json`
3. **Generated File**: Check `target/openapi/ServiceName.json` or custom path

## Error Handling

### Using RestResult and RestError

The macro uses `RestResult<T>` for all endpoints, allowing explicit HTTP status codes:

```rust
use ras_rest_core::{RestResult, RestResponse, RestError};

async fn get_user(&self, id: String) -> RestResult<User> {
    // Success with 200 OK
    Ok(RestResponse::ok(user))
    
    // Success with 201 Created
    Ok(RestResponse::created(user))
    
    // Success with custom status
    Ok(RestResponse::with_status(202, user))
    
    // Error responses
    Err(RestError::not_found("User not found"))
    Err(RestError::bad_request("Invalid user ID"))
    Err(RestError::unauthorized("Invalid token"))
    Err(RestError::forbidden("Insufficient permissions"))
    
    // Error with internal details (logged but not sent to client)
    Err(RestError::with_internal(500, "Database error", db_error))
}
```

### Client Error Handling

```typescript
try {
    const user = await client.get_users_by_id('invalid-id');
} catch (error) {
    // Error includes HTTP status and message
    console.error('Failed to get user:', error);
}
```

## Advanced Features

### 1. Usage Tracking

Track API usage for analytics or rate limiting:

```rust
.with_usage_tracker(|headers, user, method, path| async move {
    println!("API call: {} {} by {:?}", method, path, user);
    // Log to database, increment counters, etc.
})
```

### 2. Performance Monitoring

Track endpoint execution time:

```rust
.with_method_duration_tracker(|method, path, user, duration| async move {
    println!("{} {} took {:?}", method, path, duration);
    // Send metrics to monitoring system
})
```

### 3. Complex Path Parameters

Support for multiple path parameters:

```rust
PUT WITH_PERMISSIONS(["user"]) 
    users/{user_id: String}/projects/{project_id: i32}/tasks/{task_id: Uuid}(UpdateTaskRequest) 
    -> Task,
```

### 4. Multiple Permission Groups

OR logic between groups, AND logic within:

```rust
// User needs either:
// - admin permission, OR
// - both moderator AND editor permissions, OR  
// - all three: viewer, commenter, and subscriber
WITH_PERMISSIONS(["admin"] | ["moderator", "editor"] | ["viewer", "commenter", "subscriber"])
```

## Complete Example

Here's a complete example of a task management API:

```rust
use ras_rest_macro::rest_service;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// API Types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TasksResponse {
    pub tasks: Vec<Task>,
    pub total: usize,
}

// Define REST API
rest_service!({
    service_name: TaskService,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,
    endpoints: [
        // List all tasks (public)
        GET UNAUTHORIZED tasks() -> TasksResponse,
        
        // Get specific task (public)
        GET UNAUTHORIZED tasks/{id: String}() -> Task,
        
        // Create task (requires authentication)
        POST WITH_PERMISSIONS(["user"]) tasks(CreateTaskRequest) -> Task,
        
        // Update task (owner or admin)
        PUT WITH_PERMISSIONS(["owner"] | ["admin"]) tasks/{id: String}(UpdateTaskRequest) -> Task,
        
        // Delete task (owner or admin)
        DELETE WITH_PERMISSIONS(["owner"] | ["admin"]) tasks/{id: String}() -> (),
        
        // Get user's tasks
        GET WITH_PERMISSIONS(["user"]) users/{user_id: String}/tasks() -> TasksResponse,
    ]
});

// TypeScript usage
/*
import * as api from './generated/services.gen';

const config = {
  baseUrl: 'http://localhost:3000/api/v1',
  headers: { Authorization: `Bearer ${userToken}` }
};

// Create a task
const newTask = await api.postTasks({
  ...config,
  body: {
    title: 'Complete documentation',
    description: 'Write comprehensive REST macro docs'
  }
});

// Update task
await api.putTasksId({
  ...config,
  path: { id: newTask.data.id },
  body: { completed: true }
});

// Get user's tasks
const myTasks = await api.getUsersUserIdTasks({
  ...config,
  path: { user_id: userId }
});
*/
```

## Best Practices

1. **Type Safety**: Always use strongly-typed request/response objects
2. **Error Handling**: Use appropriate HTTP status codes via `RestError`
3. **Authentication**: Implement proper JWT validation in your `AuthProvider`
4. **Documentation**: Enable OpenAPI generation for API documentation
5. **Monitoring**: Use usage and duration trackers for observability
6. **CORS**: Configure CORS appropriately for frontend clients
7. **Validation**: Validate request data in your service implementation
8. **Logging**: Log internal errors while keeping client messages generic
9. **Client Generation**: Use build.rs to generate OpenAPI spec at compile time
10. **TypeScript Setup**: Configure openapi-ts to generate clients from OpenAPI spec

## Troubleshooting

### Common Issues

1. **Missing `JsonSchema` implementation**: All types must implement `JsonSchema` for OpenAPI generation
2. **OpenAPI generation fails**: Ensure `openapi: true` is set and all types implement `JsonSchema`
3. **TypeScript generation issues**: Verify the OpenAPI spec exists at the configured path
4. **Authentication fails**: Check that your `AuthProvider` is properly configured
5. **CORS errors**: Add appropriate CORS middleware to your Axum router

### Feature Flags

Control code generation with feature flags:

```toml
[features]
default = ["server"]
server = []      # Generate server-side code
client = []      # Generate native Rust client
```

## Conclusion

The `ras-rest-macro` provides a comprehensive solution for building type-safe REST APIs in Rust with automatic client generation. By defining your API once, you get:

- Type-safe server implementation
- Native Rust client
- OpenAPI specification for TypeScript client generation
- Full type safety in TypeScript with standard JavaScript
- Built-in authentication and authorization
- Performance monitoring and usage tracking

This approach eliminates the need for manual client maintenance and ensures your API clients are always in sync with your server implementation. The shift from WASM to OpenAPI-based TypeScript generation provides significant benefits in bundle size (95% reduction), developer experience, and debugging capabilities.