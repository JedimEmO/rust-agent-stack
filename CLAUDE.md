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
- **rust-jsonrpc-macro**: A procedural macro crate for compile-time code generation of JSON-RPC interfaces. Generates type-safe RPC client/server code with authentication and axum integration.
- **rust-jsonrpc-core**: Core authentication and authorization traits for JSON-RPC services. Contains the `AuthProvider` trait and related auth types.
- **rust-jsonrpc-types**: Pure JSON-RPC 2.0 protocol types and utilities. Shared across all JSON-RPC related crates.

#### Identity Management (`crates/identity/`)
- **rust-identity-core**: Core identity provider traits and types. Defines the `IdentityProvider` trait for pluggable authentication mechanisms and `UserPermissions` trait for permission lookup.
- **rust-identity-local**: Local user identity provider with username/password authentication using Argon2 password hashing.
- **rust-identity-oauth2**: OAuth2 identity provider implementation (stub, requires completion for production use).
- **rust-identity-session**: Session management with JWT token generation and validation. Includes `SessionService` for JWT issuance with permission lookup and `JwtAuthProvider` that implements the `AuthProvider` trait.

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