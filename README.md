# Rust Agent Stack (RAS)

A comprehensive Rust framework for building type-safe, authenticated agent systems with JSON-RPC, REST APIs, and file services.

## Overview

The Rust Agent Stack provides a complete toolkit for building distributed agent systems with:
- 🔐 **Pluggable Authentication** - JWT, OAuth2, local auth with security best practices
- 🚀 **Type-Safe APIs** - Procedural macros for JSON-RPC, REST, and file services
- 🌐 **WebSocket Support** - Bidirectional real-time communication
- 📁 **File Services** - Type-safe file upload/download with streaming support
- 🎯 **Full-Stack TypeScript** - Automatic TypeScript client generation via WASM
- 🎨 **Reactive WASM UIs** - Build modern web apps with Dominator framework
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
├── core/                     # Core libraries
│   ├── ras-auth-core        # Authentication traits and types
│   ├── ras-identity-core    # Core identity provider traits
│   └── ras-observability-core # Unified observability traits
├── rpc/                     # JSON-RPC libraries
│   ├── ras-jsonrpc-types    # JSON-RPC 2.0 protocol types
│   ├── ras-jsonrpc-core     # JSON-RPC runtime support
│   ├── ras-jsonrpc-macro    # JSON-RPC service macro
│   └── bidirectional/       # WebSocket support
│       ├── ras-jsonrpc-bidirectional-types
│       ├── ras-jsonrpc-bidirectional-server
│       ├── ras-jsonrpc-bidirectional-client
│       └── ras-jsonrpc-bidirectional-macro
├── rest/                    # REST API libraries
│   ├── ras-rest-core        # REST types and utilities
│   ├── ras-rest-macro       # REST service macro
│   └── ras-file-macro       # File upload/download macro
├── identity/                # Identity providers
│   ├── ras-identity-local   # Username/password auth
│   ├── ras-identity-oauth2  # OAuth2 with PKCE support
│   └── ras-identity-session # JWT session management
├── observability/           # Monitoring and metrics
│   └── ras-observability-otel # OpenTelemetry implementation
├── specs/                   # Specification types
│   └── openrpc-types        # OpenRPC 1.3.2 spec types
└── tools/                   # Development tools
    └── openrpc-to-bruno     # Convert OpenRPC to Bruno
examples/                    # Example applications
├── basic-jsonrpc/           # JSON-RPC service demo
├── bidirectional-chat/      # Real-time chat system
├── file-service-example/    # File upload/download demo
├── file-service-wasm/       # File service with TypeScript
├── oauth2-demo/             # OAuth2 authentication flow
├── rest-api-demo/           # REST API example
├── rest-wasm-example/       # REST with TypeScript client
└── wasm-ui-demo/            # Dominator WASM UI
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

Build RESTful services with automatic OpenAPI documentation and TypeScript client generation:

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
        GET WITH_PERMISSIONS(["user"]) users/{id: String}() -> User,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: String}() -> (),
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

### Type-Safe File Services

Build file upload/download services with streaming support:

```rust
use ras_file_macro::file_service;

file_service!({
    service_name: DocumentService,
    base_path: "/api/documents",
    body_limit: 52428800,  // 50MB
    endpoints: [
        UPLOAD WITH_PERMISSIONS(["user"]) upload() -> FileMetadata,
        DOWNLOAD UNAUTHORIZED download/{file_id: String}(),
    ]
});
```

### TypeScript Client Generation

All service macros support automatic TypeScript client generation:

```typescript
// Auto-generated TypeScript client
import { WasmUserServiceClient } from './pkg/user_api';

const client = new WasmUserServiceClient('http://localhost:3000');
client.set_bearer_token('your-jwt-token');

const users = await client.get_users();
const user = await client.create_user({ name: 'Alice', email: 'alice@example.com' });
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

### [Basic JSON-RPC](examples/basic-jsonrpc/)
Simple task management API demonstrating authentication and OpenTelemetry metrics.

### [OAuth2 Demo](examples/oauth2-demo/)
Full-stack OAuth2 implementation with PKCE flow and role-based permissions.

### [Bidirectional Chat](examples/bidirectional-chat/)
Real-time chat system with WebSocket communication, TUI client, and persistence.

### [File Service Example](examples/file-service-example/)
File upload/download service with streaming support and authentication.

### [File Service WASM](examples/file-service-wasm/)
Full-stack file service with TypeScript client and React frontend.

### [REST API Demo](examples/rest-api-demo/)
RESTful API with OpenAPI documentation, Swagger UI, and Prometheus metrics.

### [REST WASM Example](examples/rest-wasm-example/)
REST API with auto-generated TypeScript client and web UI.

### [WASM UI Demo](examples/wasm-ui-demo/)
Reactive web UI with Dominator framework, glass morphism design.

## Documentation

Detailed documentation is available in the `documentation/` directory:
- [REST Macro Guide](documentation/ras-rest-macro.md) - Complete REST API documentation
- [File Service Guide](documentation/ras-file-macro.md) - File upload/download services
- [Identity Providers](documentation/ras-identity.md) - Authentication system guide
- [Observability](documentation/ras-observability.md) - Metrics and monitoring

## Built-in Features

### Authentication & Security
- **Timing Attack Resistance** - Constant-time operations for authentication
- **Username Enumeration Prevention** - Uniform error responses
- **Rate Limiting** - Concurrent request limiting (5 attempts)
- **Secure Password Storage** - Argon2 hashing with proper salts
- **JWT Best Practices** - Configurable algorithms and secrets
- **PKCE OAuth2** - Proof Key for Code Exchange by default
- **Session Management** - JWT-based sessions with revocation support

### Observability

Add production-ready metrics with minimal configuration:

```rust
use ras_observability_otel::standard_setup;

// Set up OpenTelemetry with Prometheus
let otel = standard_setup("my-service")?;

// Use with service builders
let service = MyServiceBuilder::new(impl)
    .with_usage_tracker(otel.usage_tracker())
    .with_method_duration_tracker(otel.duration_tracker())
    .build();

// Metrics available at /metrics endpoint
```

Features:
- Unified metrics for JSON-RPC, REST, and file services
- Request counting, duration tracking, user activity
- Zero-config Prometheus integration
- Extensible trait-based design

### TypeScript/WASM Support

All service macros support automatic TypeScript client generation:
- Type-safe API calls with full IntelliSense
- Automatic error handling and retries
- Bearer token management
- Works in browsers and Node.js
- Zero runtime overhead with WASM

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
