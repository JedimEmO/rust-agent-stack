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
- **rust-jsonrpc-macro**: A procedural macro crate for compile-time code generation of JSON-RPC interfaces. Generates type-safe RPC client/server code with authentication and axum integration.
- **rust-jsonrpc-core**: Core authentication and authorization traits for JSON-RPC services. Contains the `AuthProvider` trait and related auth types.
- **rust-jsonrpc-types**: Pure JSON-RPC 2.0 protocol types and utilities. Shared across all JSON-RPC related crates.

### Key Design Decisions
1. **Procedural Macro Architecture**: Using proc-macros for JSON-RPC suggests focus on ergonomic, type-safe RPC interfaces with compile-time validation
2. **Workspace-First**: Structure anticipates multiple related crates sharing dependencies
3. **Agent Stack Focus**: Repository name indicates this is part of a larger agent system, with JSON-RPC as the communication protocol

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

#### Development Notes
- Edition 2024 is used (cutting edge Rust)
- All crates follow the same version (0.1.0) and edition (2024)
- Procedural macro crate can ONLY export macros, not runtime types or functions