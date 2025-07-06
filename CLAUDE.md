# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p <crate-name>  # e.g., cargo build -p ras-auth-core

# Run tests
cargo test

# Run tests for specific crate
cargo test -p <crate-name>  # e.g., cargo test -p ras-auth-core

# Run example applications
cargo run -p google-oauth-example
cargo run -p bidirectional-chat-server
cargo run -p bidirectional-chat-tui
cargo run -p basic-jsonrpc-service
cargo run -p rest-service-example

# Build and run WASM example (dominator-example)
cd examples/dominator-example
npm ci && npm start

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Run specific test
cargo test test_name

# Build release version
cargo build --release
```

## Architecture Overview

This is a Rust workspace project for building an agent stack with JSON-RPC communication capabilities.

### Workspace Structure
- Uses Cargo workspace with resolver version 3 (latest)
- Organized under `crates/` with functional grouping:
  - `core/` - Shared foundational traits and types
  - `rpc/` - JSON-RPC protocol implementation
  - `rest/` - REST API implementation
  - `identity/` - Identity and authentication providers
  - `observability/` - Monitoring and metrics
  - `specs/` - Specification types (OpenRPC, etc.)
  - `tools/` - Development tools and utilities

### Current Crates

#### Core Libraries (`crates/core/`)
- **ras-auth-core**: Shared authentication traits and types (`AuthProvider`, `AuthenticatedUser`, `AuthError`) used across JSON-RPC and REST services
- **ras-identity-core**: Core `IdentityProvider` and `UserPermissions` traits
- **ras-observability-core**: Core observability traits

#### RPC Libraries (`crates/rpc/`)
- **ras-jsonrpc-types**: Pure JSON-RPC 2.0 protocol types and utilities
- **ras-jsonrpc-core**: Core traits and utilities for JSON-RPC services (re-exports auth types from ras-auth-core)
- **ras-jsonrpc-macro**: Procedural macro for type-safe JSON-RPC interfaces with auth integration and optional OpenRPC document generation

##### Bidirectional RPC (`crates/rpc/bidirectional/`)
- **ras-jsonrpc-bidirectional-types**: Core types for bidirectional JSON-RPC communication over WebSockets
- **ras-jsonrpc-bidirectional-server**: WebSocket server runtime with Axum integration for bidirectional JSON-RPC
- **ras-jsonrpc-bidirectional-client**: Cross-platform WebSocket client (native + WASM) for bidirectional JSON-RPC
- **ras-jsonrpc-bidirectional-macro**: Procedural macro for generating bidirectional WebSocket JSON-RPC services

#### REST Libraries (`crates/rest/`)
- **ras-rest-core**: Core REST service traits
- **ras-rest-macro**: Procedural macro for type-safe REST APIs with auth integration and OpenAPI 3.0 generation

#### Specification Types (`crates/specs/`)
- **openrpc-types**: Complete OpenRPC 1.3.2 specification types with validation, builders, and JSON Schema Draft 7 support

#### Observability (`crates/observability/`)
- **ras-observability-otel**: OpenTelemetry implementation for metrics and monitoring

#### Identity Management (`crates/identity/`)
- **ras-identity-local**: Username/password auth with Argon2 hashing
- **ras-identity-oauth2**: OAuth2 provider with Google support, PKCE, and state management
- **ras-identity-session**: JWT session management and `JwtAuthProvider` implementation

#### Development Tools (`crates/tools/`)
- **openrpc-to-bruno**: CLI tool to convert OpenRPC specifications to Bruno API collections (both binary and library)

#### Examples (`examples/`)
- **google-oauth-example**: Full-stack OAuth2 demo with backend API and interactive frontend
- **bidirectional-chat**: Real-time chat system demonstrating bidirectional JSON-RPC over WebSockets
  - **bidirectional-chat-server**: WebSocket server with persistence and room management
  - **bidirectional-chat-tui**: Terminal UI client with ratatui interface
  - **bidirectional-chat-api**: Shared types and service definitions
- **basic-jsonrpc-api**: Shared API definitions for basic JSON-RPC service with client/server features
- **basic-jsonrpc-service**: Simple JSON-RPC service implementation with OpenTelemetry metrics integration
- **rest-service-example**: REST API example using ras-rest-macro with authentication and Prometheus metrics
- **dominator-example**: Full-stack WASM application using Dominator framework with reactive UI
  - Real-time task management with authentication
  - Glass morphism UI design with dwind styling
  - WebSocket JSON-RPC client connecting to basic-jsonrpc-service
  - Features: login, task CRUD, dashboard stats, dark theme

### Key Design Decisions
1. **Procedural Macro Architecture**: Using proc-macros for JSON-RPC suggests focus on ergonomic, type-safe RPC interfaces with compile-time validation
2. **Workspace-First**: Structure anticipates multiple related crates sharing dependencies
3. **Agent Stack Focus**: Repository name indicates this is part of a larger agent system, with JSON-RPC as the communication protocol
4. **Pluggable Identity Providers**: Authentication is decoupled using the `IdentityProvider` trait, allowing flexible auth mechanisms (local users, OAuth2, etc.)
5. **JWT-based Sessions**: Session management uses JWTs with configurable secrets and TTL, enabling stateless authentication across services
6. **WASM Support**: First-class support for WebAssembly client applications using Dominator framework
7. **Observability**: Built-in OpenTelemetry integration for metrics and monitoring

### Integration Points
- **MCP (Model Context Protocol)**: Use the Context7 tool to find up-to-date documentation for dependencies.
- **WASM/Web Integration**: Full WebAssembly support with Dominator framework for reactive web UIs
- **Metrics/Monitoring**: OpenTelemetry integration with Prometheus exporter support

### Development Guidelines

#### Workspace Dependencies
- **ALWAYS use workspace dependencies** for shared crates (axum, serde, tokio, etc.)
- Add dependencies to `[workspace.dependencies]` in the root Cargo.toml
- Reference them with `{ workspace = true }` in individual crate Cargo.toml files
- Use path dependencies for internal crates: `{ path = "../crate-name" }`

#### WASM/UI Dependencies (for Dominator-based applications)
- **dominator**: Reactive UI framework for building WebAssembly applications
- **dwind/dwind-macros**: CSS-in-Rust styling framework with Tailwind-like utilities
- **dwui**: UI component library built on top of Dominator
- **futures-signals-component-macro**: Reactive component macros for state management
- **wasm-bindgen**: Core WASM/JS interop library
- **wasm-pack**: Tool for building and packaging WASM modules

#### Crate Organization
- **Core** (`crates/core/`):
  - **ras-auth-core**: Shared authentication types and traits, minimal dependencies (serde, thiserror)
  - **ras-identity-core**: Core traits only, minimal dependencies
  - **ras-observability-core**: Core observability traits
- **RPC** (`crates/rpc/`):
  - **ras-jsonrpc-types**: Pure protocol types, minimal dependencies (only serde)
  - **ras-jsonrpc-core**: JSON-RPC runtime support, depends on auth-core and types crate
  - **ras-jsonrpc-macro**: Procedural macro only, depends on syn/quote for parsing
  - **Bidirectional** (`crates/rpc/bidirectional/`): WebSocket-based bidirectional RPC implementation
- **REST** (`crates/rest/`):
  - **ras-rest-core**: Core REST traits
  - **ras-rest-macro**: REST procedural macro, depends on auth-core for shared types
- **Identity** (`crates/identity/`):
  - **ras-identity-local/oauth2**: Specific provider implementations, depend on core
  - **ras-identity-session**: JWT session management, integrates with both identity and auth-core
- **Observability** (`crates/observability/`):
  - **ras-observability-otel**: OpenTelemetry implementation
- **Specs** (`crates/specs/`):
  - **openrpc-types**: OpenRPC specification types

#### Development Notes
- Edition 2024 is used (cutting edge Rust)
- All crates follow the same version (0.1.0) and edition (2024)
- Procedural macro crate can ONLY export macros, not runtime types or functions

#### Critical Development Rules (Based on Sprint Retrospectives)
1. **Test Early, Test Often**: ALWAYS run `cargo build` and `cargo test` immediately after any implementation - never assume code will compile
2. **Specification First**: When implementing standards/specs, ask for specification location/requirements before starting any research or implementation
3. **Incremental Implementation**: For complex features, break into smaller phases with compilation/testing checkpoints rather than implementing everything at once
4. **Macro Testing**: Always test generated macro code with real routing scenarios and actual parameters (especially base_url)
5. **End-to-End Validation**: Test complete flows during initial implementation, not as an afterthought

#### Common Pitfalls
- **Axum Router Nesting**: Use `.nest("/api", router)` not `.merge(router.nest("/api", Router::new()))` - the latter creates invalid nesting syntax
- **Macro Base URL**: Always test generated macro code with the actual base_url parameter to catch routing issues
- **String Type Mismatches**: Watch for bon builder string literal type mismatches (String vs &str)
- **Move Semantics**: Pay attention to type annotations and move semantics in complex macro-generated code
- **Bidirectional Macro**: The `openrpc` field is NOT supported in `jsonrpc_bidirectional_service!` - always remove it
- **Generated Type Names**: Macro generates trait named `{ServiceName}Service` (e.g., `ChatServiceService` for `ChatService`)
- **Arc in Handler State**: When using Arc<T> in Axum handlers, ensure proper state tuple handling - may require multiple iterations
- **Module Visibility**: Always check module exports are public when creating new modules that need external access
- **Private Field Access**: Identity providers may have private fields - plan for workarounds when syncing between endpoints

#### OpenRPC Document Generation
The `jsonrpc_service` macro supports optional OpenRPC document generation for API documentation:

```rust
// Enable OpenRPC with default output (target/openrpc/{service_name}.json)
jsonrpc_service!({
    service_name: MyService,
    openrpc: true,
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS(["admin"]) delete_everything(()) -> (),
    ]
});

// Enable OpenRPC with custom output path
jsonrpc_service!({
    service_name: MyService,
    openrpc: { output: "docs/api.json" },
    methods: [...]
});
```

**Key Features:**
- **Per-service control**: Each macro invocation can independently enable/disable OpenRPC generation
- **Schema generation**: Uses `schemars` crate to generate JSON Schema for request/response types
- **Authentication metadata**: Includes `x-authentication` and `x-permissions` extensions in the OpenRPC document
- **Compile-time generation**: Documents are generated during compilation and written to specified paths

**Requirements:**
- All request/response types must implement `schemars::JsonSchema` trait
- Generated functions: `generate_{service_name}_openrpc()` and `generate_{service_name}_openrpc_to_file()`

#### Bidirectional JSON-RPC over WebSockets
The `jsonrpc_bidirectional_service!` macro enables type-safe, bidirectional JSON-RPC communication over WebSockets:

```rust
jsonrpc_bidirectional_service!({
    service_name: ChatService,
    // NOTE: openrpc field is NOT supported - remove if present
    
    // Client -> Server methods (with authentication/permissions)
    client_to_server: [
        WITH_PERMISSIONS(["user"]) send_message(SendMessageRequest) -> SendMessageResponse,
        WITH_PERMISSIONS(["admin"]) kick_user(KickUserRequest) -> KickUserResponse,
    ],
    
    // Server -> Client notifications (no response expected)  
    server_to_client: [
        message_received(MessageReceivedNotification),
        user_joined(UserJoinedNotification),
        user_left(UserLeftNotification),
    ]
});
```

**Generated Components:**
- **Server trait**: Service implementation interface with handler methods
- **Server builder**: WebSocket service configuration with Axum integration
- **Client struct**: Type-safe client with method calls and notification handlers
- **Message enums**: Type-safe communication in both directions
- **OpenRPC docs**: Optional documentation generation for client_to_server methods

**Authentication Model:**
- JWT authentication during WebSocket handshake with multiple header support:
  - Bearer token in Authorization header
  - WebSocket protocol header (`sec-websocket-protocol: token.{jwt_token}`)
  - X-Auth-Token header
- Persistent auth context for connection lifetime
- Permission-based access control for client_to_server methods
- Automatic connection cleanup on token expiration

**Key Features:**
- Cross-platform client support (native + WASM using conditional compilation)
- Connection management with subscription/broadcast patterns
- Heartbeat/keepalive for connection health
- Automatic reconnection with exponential backoff
- Integration with existing auth system (ras-auth-core, ras-identity-*)

#### Testing Guidelines  
- **Security-First**: Include security testing (timing attacks, username enumeration) from initial implementation
- **End-to-End Testing**: Always test complete flows (e.g., OAuth2 flow from start to finish) during implementation
- **Macro Testing**: Test generated macro code with real routing scenarios, not just unit tests
- **Integration Testing**: Test how different crates work together, especially auth flows

### Error Handling Guidelines
- Use `thiserror` for library error handling and `anyhow` for application level errors

## Authentication Architecture

The authentication system is designed with flexibility and security in mind:

### Two-Stage Authentication Flow
1. **Identity Verification** (`IdentityProvider`): Validates credentials against various providers
   - Accepts provider-specific payloads as `JsonValue` for decoupling
   - Returns a `VerifiedIdentity` with basic user information
   - Supports provider-specific metadata storage
   
2. **Session Creation** (`SessionService`): Issues JWTs after successful identity verification
   - Generates session-specific JTIs for tracking
   - Configurable JWT secrets and TTL
   - Maintains active session registry (for revocation)
   - Session termination capability via `end_session()` method
   - Configurable JWT algorithm (not just HS256)
   - Refresh token support (configuration field available)

### Integration with JSON-RPC
- `JwtAuthProvider` implements the `AuthProvider` trait from `ras-jsonrpc-core`
- This allows JWT-based authentication to work seamlessly with the JSON-RPC macro-generated services
- The flow: Identity Provider → Session Service → JWT → JwtAuthProvider → JSON-RPC Service

### Permission System
- `UserPermissions` trait enables flexible permission lookup during session creation
- Accepts `VerifiedIdentity` and returns a list of permission strings
- Built-in implementations:
  - `NoopPermissions`: Returns no permissions (default)
  - `StaticPermissions`: Returns same permissions for all users
- Permissions are embedded in JWTs and automatically validated by `AuthProvider`
- Supports role-based access control (RBAC) patterns

### Security Considerations
- Passwords are hashed using Argon2 (industry standard)
- JWTs use configurable secrets and algorithms
- Session tracking enables token revocation with active session registry
- Provider parameters use `JsonValue` to prevent type coupling and allow flexible configuration
- Permissions are embedded in JWT claims for stateless authorization
- Rate limiting: Semaphore-based concurrent authentication attempt limiting (5 concurrent attempts)
- OAuth2 security: PKCE enforced by default, state management with TTL

#### Authentication Attack Vector Protection
- **Username Enumeration Prevention**: All authentication failures return identical `InvalidCredentials` errors regardless of whether the username exists or the password is wrong
- **Timing Attack Resistance**: Constant-time authentication using real Argon2 dummy hash for non-existent users to ensure consistent response times
- **Input Validation**: Robust handling of malformed payloads, empty credentials, and special characters
- **Brute Force Protection**: Consistent error handling across repeated authentication attempts with semaphore-based rate limiting
- **Thread Safety**: Concurrent authentication attempts are handled safely without information leakage (5 concurrent limit)
- **Security Testing**: Comprehensive test suite covering username enumeration, timing attacks, password spraying, and other common attack vectors

## Examples and Usage

### Google OAuth2 Example (`examples/google-oauth-example/`)

Full-stack OAuth2 demo with Google integration, showcasing complete authentication flow with PKCE, permission-based access control, and interactive frontend.

**Quick Start:**
```bash
# 1. Set up Google OAuth2 credentials at https://console.cloud.google.com/
# 2. Configure credentials in examples/google-oauth-example/.env
# 3. Run: cargo run -p google-oauth-example
# 4. Open browser to http://localhost:3000
```

**Key Features:** OAuth2 + PKCE, role-based permissions, JSON-RPC API, responsive web UI

### Bidirectional Chat Example (`examples/bidirectional-chat/`)

Real-time chat system showcasing WebSocket-based bidirectional JSON-RPC communication with authentication, persistence, and rich UI features.

#### Bidirectional Macro Implementation Notes
- **Required Fields**: Always include `server_to_client_calls` field even if empty (`server_to_client_calls: []`)
- **Connection Management**: Use the `ConnectionManager` directly for sending notifications, not a generated client handle
- **Persistence**: Chat history and room state can be persisted using simple JSON file storage with `serde_json`
- **Integration Testing**: Extensive test coverage including WebSocket tests, authentication flows, and concurrent user scenarios

### Basic JSON-RPC Service Example (`examples/basic-jsonrpc-service/`)

Simple service demonstrating JSON-RPC macros with authentication and metrics:

**Features:**
- Task management API with CRUD operations
- JWT authentication with local user provider
- OpenTelemetry metrics with Prometheus exporter
- API/Service separation pattern for clean architecture

### Dominator WASM Example (`examples/dominator-example/`)

Full-stack WebAssembly application showcasing reactive UI with Dominator framework:

**Quick Start:**
```bash
cd examples/dominator-example
./build.sh  # Builds WASM and starts development server
# Open browser to http://localhost:8080
```

**Key Features:**
- Reactive state management with futures-signals
- Glass morphism UI design with dwind styling
- WebSocket JSON-RPC client integration
- Real-time task management with priority levels
- Dashboard with animated statistics
- Dark theme with smooth transitions
- Responsive design for all screen sizes

**Architecture:**
- Pure client-side rendering (no SSR)
- Connects to basic-jsonrpc-service backend
- Uses bon builder pattern for API calls
- Component-based architecture with reactive signals

### Production Deployment Considerations

#### General Guidelines:
- **Security**: Use proper JWT secrets, enable CORS only for trusted origins, implement rate limiting
- **Persistence**: Configure database storage instead of JSON files
- **Scaling**: Consider Redis for pub/sub across multiple server instances
- **Monitoring**: Use structured logging with OpenTelemetry integration
- **Configuration**: Use environment-specific config files and secure credential management

#### WASM-specific Considerations:
- **Build optimization**: Use `--release` flag for production builds
- **Asset serving**: Configure CDN for WASM and static assets
- **WebSocket proxy**: Ensure proper WebSocket forwarding in reverse proxy
- **CORS configuration**: Set appropriate CORS headers for API endpoints

## UI Development with Dominator

The project includes first-class support for building reactive WebAssembly UIs using the Dominator framework:

### Key Libraries:
- **dominator**: Zero-cost reactive UI framework with signals-based state management
- **dwind**: CSS-in-Rust with Tailwind-like utilities and compile-time optimization
- **dwui**: Pre-built UI components for common patterns
- **futures-signals**: Reactive state management primitives

### Development Tips:
1. Use `Mutable` and `MutableVec` for reactive state
2. Leverage dwind's compile-time CSS generation for zero-runtime overhead
3. Component functions should return `Dom` elements
4. Use `clone!` macro for capturing state in closures
5. Prefer `html!` macro for complex DOM structures

For detailed Dominator documentation and patterns, ask the Dominator guru!