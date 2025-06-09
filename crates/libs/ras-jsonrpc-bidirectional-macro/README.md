# ras-jsonrpc-bidirectional-macro

Procedural macro for generating type-safe bidirectional JSON-RPC services over WebSockets.

This crate provides the `jsonrpc_bidirectional_service!` macro that generates both server and client code for bidirectional JSON-RPC communication, including authentication support, type-safe message enums, and optional OpenRPC documentation.

## Features

- **Server Code Generation**: Generates service traits and handlers for client-to-server JSON-RPC methods
- **Client Code Generation**: Generates type-safe client structs with method calls and notification handlers
- **Authentication Integration**: Supports JWT-based authentication with permission-based access control
- **Type Safety**: All generated code is fully type-safe with compile-time validation
- **OpenRPC Documentation**: Optional automatic OpenRPC specification generation
- **WebSocket Integration**: Works seamlessly with the bidirectional runtime crates

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
ras-jsonrpc-bidirectional-macro = { path = "path/to/ras-jsonrpc-bidirectional-macro" }

# Optional features
[features]
server = ["ras-jsonrpc-bidirectional-server"]
client = ["ras-jsonrpc-bidirectional-client"] 
openrpc = ["ras-jsonrpc-bidirectional-macro/openrpc"]
```

### Basic Example

```rust
use ras_jsonrpc_bidirectional_macro::jsonrpc_bidirectional_service;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRequest {
    pub user_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub message: String,
    pub timestamp: u64,
}

// Generate bidirectional service
jsonrpc_bidirectional_service!({
    service_name: UserService,
    openrpc: true,
    client_to_server: [
        UNAUTHORIZED get_user(UserRequest) -> UserResponse,
        WITH_PERMISSIONS(["admin"]) delete_user(UserRequest) -> bool,
        WITH_PERMISSIONS(["write"] | ["admin"]) update_user(UserRequest) -> UserResponse,
    ],
    server_to_client: [
        status_notification(StatusUpdate),
        user_updated(UserResponse),
    ]
});
```

This generates:

#### Server Side (with `#[cfg(feature = "server")]`)

```rust
// Service trait to implement
#[async_trait::async_trait]
pub trait UserServiceService: Send + Sync {
    async fn get_user(&self, request: UserRequest) -> Result<UserResponse, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_user(&self, user: &AuthenticatedUser, request: UserRequest) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_user(&self, user: &AuthenticatedUser, request: UserRequest) -> Result<UserResponse, Box<dyn std::error::Error + Send + Sync>>;
    
    // Notification methods
    async fn notify_status_notification(&self, connection_id: ConnectionId, params: StatusUpdate) -> Result<()>;
    async fn notify_user_updated(&self, connection_id: ConnectionId, params: UserResponse) -> Result<()>;
}

// Builder for WebSocket service
pub struct UserServiceBuilder<T: UserServiceService, A: AuthProvider> {
    // ...
}
```

#### Client Side (with `#[cfg(feature = "client")]`)

```rust
// Type-safe client
pub struct UserServiceClient {
    // ...
}

impl UserServiceClient {
    // Method calls
    pub async fn get_user(&self, request: UserRequest) -> ClientResult<UserResponse>;
    pub async fn delete_user(&self, request: UserRequest) -> ClientResult<bool>;
    pub async fn update_user(&self, request: UserRequest) -> ClientResult<UserResponse>;
    
    // Notification handlers
    pub fn on_status_notification<F>(&mut self, handler: F)
    where F: Fn(StatusUpdate) + Send + Sync + 'static;
    
    pub fn on_user_updated<F>(&mut self, handler: F) 
    where F: Fn(UserResponse) + Send + Sync + 'static;
    
    // Connection management
    pub async fn connect(&self) -> ClientResult<()>;
    pub async fn disconnect(&self) -> ClientResult<()>;
    pub fn is_connected(&self) -> bool;
}

// Client builder
pub struct UserServiceClientBuilder {
    // ...
}
```

### Server Implementation Example

```rust
use ras_auth_core::AuthenticatedUser;

struct MyUserService;

#[async_trait::async_trait]
impl UserServiceService for MyUserService {
    async fn get_user(&self, request: UserRequest) -> Result<UserResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Implementation
        Ok(UserResponse {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        })
    }
    
    async fn delete_user(&self, user: &AuthenticatedUser, request: UserRequest) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Check user permissions are automatically validated by the generated code
        // Implementation
        Ok(true)
    }
    
    async fn update_user(&self, user: &AuthenticatedUser, request: UserRequest) -> Result<UserResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Implementation
        Ok(UserResponse {
            name: "Updated Name".to_string(),
            email: "updated@example.com".to_string(),
        })
    }
    
    async fn notify_status_notification(&self, connection_id: ConnectionId, params: StatusUpdate) -> Result<()> {
        // Default implementation sends notification to the connection
        self.notify_status_notification(connection_id, params).await
    }
    
    async fn notify_user_updated(&self, connection_id: ConnectionId, params: UserResponse) -> Result<()> {
        // Default implementation sends notification to the connection  
        self.notify_user_updated(connection_id, params).await
    }
}

// Create and run server
#[tokio::main]
async fn main() {
    let service = MyUserService;
    let auth_provider = JwtAuthProvider::new("secret".to_string());
    
    let websocket_service = UserServiceBuilder::new(service, auth_provider)
        .require_auth(false) // Set to true to require authentication for all methods
        .build();
    
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(websocket_service);
    
    axum::serve(listener, app).await.unwrap();
}
```

### Client Usage Example

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = UserServiceClientBuilder::new("ws://localhost:8080/ws")
        .with_jwt_token("your_jwt_token".to_string())
        .build()
        .await?;
    
    // Register notification handlers
    client.on_status_notification(|status| {
        println!("Status update: {}", status.message);
    });
    
    client.on_user_updated(|user| {
        println!("User updated: {}", user.name);
    });
    
    // Connect to server
    client.connect().await?;
    
    // Make RPC calls
    let user = client.get_user(UserRequest { user_id: 123 }).await?;
    println!("User: {:?}", user);
    
    let deleted = client.delete_user(UserRequest { user_id: 123 }).await?;
    println!("Deleted: {}", deleted);
    
    Ok(())
}
```

## Macro Syntax

```rust
jsonrpc_bidirectional_service!({
    service_name: ServiceName,
    openrpc: true | false | { output: "path/to/output.json" },
    client_to_server: [
        UNAUTHORIZED method_name(RequestType) -> ResponseType,
        WITH_PERMISSIONS(["perm1", "perm2"]) method_name(RequestType) -> ResponseType,
        WITH_PERMISSIONS(["perm1"] | ["perm2"]) method_name(RequestType) -> ResponseType, // OR groups
    ],
    server_to_client: [
        notification_name(NotificationType),
        another_notification(AnotherType),
    ]
});
```

### Authentication

- `UNAUTHORIZED`: No authentication required
- `WITH_PERMISSIONS(["perm1", "perm2"])`: User must have ALL listed permissions (AND logic)
- `WITH_PERMISSIONS(["perm1"] | ["perm2"])`: User must have permissions from ANY group (OR logic between groups, AND within groups)

### OpenRPC Generation

When `openrpc: true` is specified, the macro generates OpenRPC documentation:

- **Output**: `target/openrpc/{service_name}.json` by default
- **Custom path**: Use `openrpc: { output: "custom/path.json" }`
- **Requires**: All request/response types must implement `schemars::JsonSchema`
- **Features**: Include `openrpc` feature in your `Cargo.toml`

Generated functions:
- `generate_{service_name}_openrpc()` -> Returns OpenRPC document
- `generate_{service_name}_openrpc_to_file()` -> Writes to file

## Requirements

All request, response, and notification parameter types must implement:
- `serde::Serialize` + `serde::Deserialize`
- `Send` + `Sync` + `'static`
- `schemars::JsonSchema` (if using OpenRPC generation)

## Generated Code Structure

The macro generates code conditionally compiled based on features:

- `#[cfg(feature = "server")]`: Server traits, handlers, and builders
- `#[cfg(feature = "client")]`: Client structs, builders, and message enums  
- `#[cfg(feature = "openrpc")]`: OpenRPC generation functions

This allows consuming crates to enable only the functionality they need.

## Error Handling

Generated code provides comprehensive error handling:

- **Authentication errors**: Automatic JWT validation and permission checking
- **Serialization errors**: Type-safe JSON conversion with helpful error messages
- **Connection errors**: WebSocket connection management and recovery
- **Method errors**: User-defined error types from service implementations

## Integration with Runtime Crates

This macro works with the following runtime crates:

- `ras-jsonrpc-bidirectional-types`: Core types and traits
- `ras-jsonrpc-bidirectional-server`: Server-side WebSocket handling  
- `ras-jsonrpc-bidirectional-client`: Client-side WebSocket communication
- `ras-auth-core`: Authentication provider traits
- `ras-jsonrpc-types`: JSON-RPC 2.0 protocol types