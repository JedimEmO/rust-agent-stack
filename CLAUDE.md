# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p rust-jsonrpc-macro
cargo build -p rust-jsonrpc-core
cargo build -p rust-jsonrpc-types

# Run tests
cargo test

# Run tests for specific crate
cargo test -p rust-jsonrpc-macro
cargo test -p rust-jsonrpc-core
cargo test -p rust-jsonrpc-types

# Run example applications
cargo run -p google-oauth-example

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
- Organized under `crates/` with subcategories:
  - `libs/` - Library crates
  - Future expansion likely: `apps/`, `services/`

### Current Crates

#### JSON-RPC Libraries (`crates/libs/`)
- **rust-jsonrpc-macro**: Procedural macro for type-safe JSON-RPC interfaces with auth integration
- **rust-jsonrpc-core**: Core `AuthProvider` trait and auth types for JSON-RPC services  
- **rust-jsonrpc-types**: Pure JSON-RPC 2.0 protocol types and utilities

#### Identity Management (`crates/identity/`)
- **rust-identity-core**: Core `IdentityProvider` and `UserPermissions` traits
- **rust-identity-local**: Username/password auth with Argon2 hashing
- **rust-identity-oauth2**: OAuth2 provider with Google support, PKCE, and state management
- **rust-identity-session**: JWT session management and `JwtAuthProvider` implementation

#### Examples (`examples/`)
- **google-oauth-example**: Full-stack OAuth2 demo with backend API and interactive frontend

### Key Design Decisions
1. **Procedural Macro Architecture**: Using proc-macros for JSON-RPC suggests focus on ergonomic, type-safe RPC interfaces with compile-time validation
2. **Workspace-First**: Structure anticipates multiple related crates sharing dependencies
3. **Agent Stack Focus**: Repository name indicates this is part of a larger agent system, with JSON-RPC as the communication protocol
4. **Pluggable Identity Providers**: Authentication is decoupled using the `IdentityProvider` trait, allowing flexible auth mechanisms (local users, OAuth2, etc.)
5. **JWT-based Sessions**: Session management uses JWTs with configurable secrets and TTL, enabling stateless authentication across services

### Integration Points
- **MCP (Model Context Protocol)**: Configured with Context7 for enhanced documentation access during development
- **Potential JS/TS Integration**: `.gitignore` includes `node_modules/`, suggesting possible JavaScript/TypeScript components

### Development Guidelines

#### Workspace Dependencies
- **ALWAYS use workspace dependencies** for shared crates (axum, serde, tokio, etc.)
- Add dependencies to `[workspace.dependencies]` in the root Cargo.toml
- Reference them with `{ workspace = true }` in individual crate Cargo.toml files
- Use path dependencies for internal crates: `{ path = "../crate-name" }`

#### Crate Organization
- **rust-jsonrpc-types**: Pure protocol types, minimal dependencies (only serde)
- **rust-jsonrpc-core**: Auth traits and runtime support, depends on types crate
- **rust-jsonrpc-macro**: Procedural macro only, depends on syn/quote for parsing
- **rust-identity-core**: Core traits only, minimal dependencies
- **rust-identity-local/oauth2**: Specific provider implementations, depend on core
- **rust-identity-session**: JWT session management, integrates with both identity and jsonrpc-core

#### Development Notes
- Edition 2024 is used (cutting edge Rust)
- All crates follow the same version (0.1.0) and edition (2024)
- Procedural macro crate can ONLY export macros, not runtime types or functions

#### Common Pitfalls
- **Axum Router Nesting**: Use `.nest("/api", router)` not `.merge(router.nest("/api", Router::new()))` - the latter creates invalid nesting syntax
- **Macro Base URL**: Always test generated macro code with the actual base_url parameter to catch routing issues

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
   
2. **Session Creation** (`SessionService`): Issues JWTs after successful identity verification
   - Generates session-specific JTIs for tracking
   - Configurable JWT secrets and TTL
   - Maintains active session registry (for revocation)

### Integration with JSON-RPC
- `JwtAuthProvider` implements the `AuthProvider` trait from `rust-jsonrpc-core`
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
- JWTs use configurable secrets (HS256 by default)
- Session tracking enables token revocation
- Provider parameters use `JsonValue` to prevent type coupling and allow flexible configuration
- Permissions are embedded in JWT claims for stateless authorization

#### Authentication Attack Vector Protection
- **Username Enumeration Prevention**: All authentication failures return identical `InvalidCredentials` errors regardless of whether the username exists or the password is wrong
- **Timing Attack Resistance**: Constant-time authentication using real Argon2 dummy hash for non-existent users to ensure consistent response times
- **Input Validation**: Robust handling of malformed payloads, empty credentials, and special characters
- **Brute Force Protection**: Consistent error handling across repeated authentication attempts
- **Thread Safety**: Concurrent authentication attempts are handled safely without information leakage
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