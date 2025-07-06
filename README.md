# Rust Agent Stack (RAS)

A comprehensive Rust framework for building type-safe, authenticated agent systems with JSON-RPC and REST APIs.

## Overview

The Rust Agent Stack provides a complete toolkit for building distributed agent systems with:
- 🔐 **Pluggable Authentication** - JWT, OAuth2, local auth with security best practices
- 🚀 **Type-Safe APIs** - Trait-based procedural macros for JSON-RPC and REST
- 🌐 **WebSocket Support** - Bidirectional real-time communication
- 🎯 **WASM Support** - Build reactive web UIs with Dominator framework
- 📊 **Observability** - Built-in OpenTelemetry and Prometheus metrics
- 📝 **API Documentation** - Automatic OpenRPC and OpenAPI generation
- ✅ **Compile-Time Safety** - All endpoints must be implemented

## Quick Start

```bash
# Clone the repository
git clone https://github.com/yourusername/rust-agent-stack.git
cd rust-agent-stack

# Build the entire workspace
cargo build

# Run an example service
cargo run -p basic-jsonrpc-service

# In another terminal, run the WASM UI example
cd examples/dominator-example
./build.sh
# Open http://localhost:8080
```

## Architecture

RAS is organized as a Cargo workspace with the following structure:

```
crates/
├── libs/                     # Core libraries
│   ├── ras-auth-core        # Authentication traits and types
│   ├── ras-jsonrpc-core     # JSON-RPC runtime support
│   ├── ras-jsonrpc-macro    # JSON-RPC service macro
│   ├── ras-jsonrpc-types    # JSON-RPC protocol types
│   ├── ras-jsonrpc-bidirectional-* # WebSocket support
│   ├── ras-rest-core        # REST types and utilities
│   ├── ras-rest-macro       # REST service macro
│   ├── ras-observability-core # Unified observability traits
│   ├── ras-observability-otel # OpenTelemetry implementation
│   └── openrpc-types        # OpenRPC specifications
├── identity/                # Identity providers
│   ├── ras-identity-core    # Core identity traits
│   ├── ras-identity-local   # Username/password auth
│   ├── ras-identity-oauth2  # OAuth2 with PKCE support
│   └── ras-identity-session # JWT session management
└── tools/                   # Development tools
    └── openrpc-to-bruno     # Convert OpenRPC to Bruno collections
examples/                    # Example applications
```

## Key Features

### Type-Safe JSON-RPC Services

Define services with compile-time type checking:

```rust
use ras_jsonrpc_macro::jsonrpc_service;

jsonrpc_service!({
    service_name: TaskService,
    openrpc: true,  // Generate OpenRPC docs
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS(["user"]) create_task(CreateTaskRequest) -> Task,
        WITH_PERMISSIONS(["admin"]) delete_all_tasks(()) -> (),
    ]
});

// Implement the generated trait
struct TaskServiceImpl { /* ... */ }

#[async_trait::async_trait]
impl TaskServiceHandler for TaskServiceImpl {
    async fn sign_in(&self, request: SignInRequest) -> JsonRpcResult<SignInResponse> {
        // Your implementation
    }
    // ... other methods
}

// Use with the builder
let service = TaskService::builder()
    .auth_provider(JwtAuthProvider::new())
    .build(Arc::new(TaskServiceImpl { /* ... */ }));
```

### Type-Safe REST APIs

Build RESTful services with automatic OpenAPI documentation:

```rust
use ras_rest_macro::rest_service;

rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,  // Serve Swagger UI at /api/v1/docs
    endpoints: [
        GET UNAUTHORIZED users() -> UsersResponse,
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> UserResponse,
        GET WITH_PERMISSIONS(["user"]) users/{id: i32}() -> UserResponse,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: i32}() -> (),
    ]
});

// Implement the generated trait
struct UserServiceImpl { /* ... */ }

#[async_trait::async_trait]
impl UserServiceTrait for UserServiceImpl {
    async fn get_users(&self) -> RestResult<UsersResponse> {
        // Your implementation
    }
    // ... other methods
}

// Build the Axum router
let app = UserServiceBuilder::new(UserServiceImpl { /* ... */ })
    .auth_provider(jwt_auth_provider)
    .build();
```

### Bidirectional WebSocket Communication

Real-time bidirectional messaging with authentication:

```rust
use ras_jsonrpc_bidirectional_macro::jsonrpc_bidirectional_service;

jsonrpc_bidirectional_service!({
    service_name: ChatService,

    client_to_server: [
        WITH_PERMISSIONS(["user"]) send_message(SendMessageRequest) -> SendMessageResponse,
    ],

    server_to_client: [
        message_received(MessageReceivedNotification),
        user_joined(UserJoinedNotification),
    ]
});
```

### Reactive WASM UIs

Build modern web applications with Dominator:

```rust
use dominator::{html, Dom};
use futures_signals::signal::Mutable;

fn create_task_list(tasks: MutableVec<Task>) -> Dom {
    html!("div", {
        .class("task-list")
        .children_signal_vec(tasks.signal_vec_cloned()
            .map(|task| render_task(task)))
    })
}
```

## Examples

### [Basic JSON-RPC Service](examples/basic-jsonrpc-service/)
Simple task management API demonstrating authentication and OpenTelemetry metrics.

### [Google OAuth Example](examples/google-oauth-example/)
Full-stack OAuth2 implementation with PKCE flow and role-based permissions.

### [Bidirectional Chat](examples/bidirectional-chat/)
Real-time chat system with WebSocket communication, TUI client, and persistence.

### [Dominator WASM Example](examples/dominator-example/)
Reactive web UI with glass morphism design, connecting to JSON-RPC backend.

### [REST Service Example](examples/rest-service-example/)
RESTful API with OpenAPI documentation and Prometheus metrics.

## Security Features

- **Timing Attack Resistance** - Constant-time operations for authentication
- **Username Enumeration Prevention** - Uniform error responses
- **Rate Limiting** - Built-in concurrent request limiting
- **Secure Password Storage** - Argon2 hashing with proper salts
- **JWT Best Practices** - Configurable algorithms and secrets
- **PKCE OAuth2** - Proof Key for Code Exchange by default

### Built-in Observability

Add production-ready metrics with one line of code:

```rust
use ras_observability_otel::standard_setup;

// Set up OpenTelemetry with Prometheus
let otel = standard_setup("my-service")?;

// Use with service builders
let service = MyServiceBuilder::new()
    .with_usage_tracker(/* ... */)
    .with_method_duration_tracker(/* ... */)
    .build();

// Metrics available at /metrics endpoint
```

Features:
- Unified metrics for both REST and JSON-RPC
- Request counting, duration tracking, user activity
- Zero-config Prometheus integration
- Extensible trait-based design

## Development

See [CLAUDE.md](CLAUDE.md) for detailed development guidelines and architecture decisions.

### Building

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run all tests
cargo test

# Run specific crate tests
cargo test -p ras-auth-core
```

### Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate OpenRPC documentation (when enabled)
cargo build  # OpenRPC files generated in target/openrpc/
```

## Contributing

Contributions are welcome! Please read our contributing guidelines and code of conduct.

### Development Setup

1. Install Rust (latest stable)
2. Install wasm-pack for WASM examples: `cargo install wasm-pack`
3. Clone the repository
4. Run `cargo build` to verify setup

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

Built with these excellent Rust crates:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Tokio](https://tokio.rs/) - Async runtime
- [Dominator](https://github.com/Pauan/rust-dominator) - WASM UI framework
- [Tungstenite](https://github.com/snapview/tungstenite-rs) - WebSocket implementation
- [jsonwebtoken](https://github.com/Keats/jsonwebtoken) - JWT support
- [async-trait](https://github.com/dtolnay/async-trait) - Async traits
