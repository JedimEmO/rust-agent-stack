# Rust Agent Stack Examples

This directory contains example applications demonstrating various features of the Rust Agent Stack.

## Overview

The examples are organized to showcase different aspects of the framework:
- **JSON-RPC services** with authentication and OpenRPC documentation
- **REST APIs** with OpenAPI generation
- **WebSocket-based bidirectional communication**
- **OAuth2 authentication flows**
- **WebAssembly UI applications**

## Examples

### Basic JSON-RPC (`basic-jsonrpc/`)

Demonstrates core JSON-RPC functionality with a simple task management service.

- **api/**: Shared API definitions using the `jsonrpc_service!` macro
- **service/**: HTTP server implementation with:
  - JWT authentication using local user provider
  - OpenTelemetry metrics integration
  - Prometheus metrics endpoint
  - OpenRPC document generation

**Quick Start:**
```bash
cd examples/basic-jsonrpc/service
cargo run
# API available at http://localhost:3000
# Metrics at http://localhost:3000/metrics
```

### Bidirectional Chat (`bidirectional-chat/`)

Real-time chat application showcasing WebSocket-based bidirectional JSON-RPC.

- **api/**: Shared WebSocket RPC definitions using `jsonrpc_bidirectional_service!`
- **server/**: Chat server with:
  - Multi-room support
  - Message persistence
  - User presence tracking
  - Typing indicators
- **tui/**: Terminal UI client with ratatui interface

**Quick Start:**
```bash
# Terminal 1: Start server
cd examples/bidirectional-chat/server
cargo run

# Terminal 2: Start TUI client
cd examples/bidirectional-chat/tui
cargo run
```

### OAuth2 Demo (`oauth2-demo/`)

Full OAuth2 authentication flow implementation with Google as the provider.

- **api/**: OAuth2-protected API definitions
- **server/**: Complete OAuth2 server with:
  - Authorization code flow with PKCE
  - State management for security
  - JWT session creation after successful auth
  - Static file serving for frontend
  - Role-based permissions

**Quick Start:**
```bash
# 1. Set up Google OAuth2 credentials at https://console.cloud.google.com/
# 2. Configure credentials in examples/oauth2-demo/.env
cd examples/oauth2-demo/server
cargo run
# Open browser to http://localhost:3000
```

### REST API Demo (`rest-api-demo/`)

Demonstrates the REST macro for building type-safe REST APIs.

- OpenAPI 3.0 document generation
- JWT authentication with local users
- Prometheus metrics integration
- CRUD operations for task management
- Request/response validation

**Quick Start:**
```bash
cd examples/rest-api-demo
cargo run
# API at http://localhost:3000
# OpenAPI docs generated at startup
```

### WASM UI Demo (`wasm-ui-demo/`)

Full-stack WebAssembly application using the Dominator reactive framework.

- Glass morphism UI design with dwind styling
- Real-time task management
- WebSocket connection to basic-jsonrpc-service
- Reactive state management with futures-signals
- Dark theme support
- Responsive design

**Quick Start:**
```bash
# Terminal 1: Start the backend service
cd examples/basic-jsonrpc/service
cargo run

# Terminal 2: Build and serve the WASM app
cd examples/wasm-ui-demo
npm ci
npm start
# Open browser to http://localhost:8080
```

## Architecture Patterns

### Multi-Crate Examples
Examples like `basic-jsonrpc/`, `bidirectional-chat/`, and `oauth2-demo/` are structured as multi-crate workspaces:
- `api/`: Shared type definitions and service traits
- `server/`: Backend implementation
- `client/` or `tui/`: Frontend implementation

This separation allows:
- Code reuse between client and server
- Independent versioning
- Clear API contracts

### Single-Crate Examples
Examples like `rest-api-demo/` and `wasm-ui-demo/` are self-contained:
- Simpler structure for focused demonstrations
- All code in one crate
- Easier to understand for specific features

## Common Features

### Authentication
Most examples demonstrate authentication patterns:
- **Local users**: Username/password with Argon2 hashing
- **OAuth2**: External provider integration
- **JWT sessions**: Stateless authentication tokens
- **Permissions**: Role-based access control

### Observability
Examples include monitoring capabilities:
- OpenTelemetry integration
- Prometheus metrics export
- Structured logging with tracing

### Documentation
API documentation is auto-generated:
- **OpenRPC**: For JSON-RPC services
- **OpenAPI**: For REST APIs
- Generated at compile-time or runtime

## Development Tips

1. **Environment Variables**: Check `.env.example` files for required configuration
2. **Dependencies**: Examples use workspace dependencies from the root `Cargo.toml`
3. **Cross-Example Integration**: Some examples (like wasm-ui-demo) connect to other example services
4. **Generated Files**: OpenRPC/OpenAPI documents are typically generated in `target/` directories

## Testing

Each example includes various levels of testing:
- Unit tests in the source files
- Integration tests in `tests/` directories
- Manual testing instructions in example-specific READMEs

Run all example tests:
```bash
cargo test --workspace --examples
```