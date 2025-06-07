# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p rust-jsonrpc-macro

# Run tests
cargo test

# Run tests for specific crate
cargo test -p rust-jsonrpc-macro

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
- **rust-jsonrpc-macro**: A procedural macro crate (currently empty) intended to provide compile-time code generation for JSON-RPC interfaces. This will likely generate type-safe RPC client/server code.

### Key Design Decisions
1. **Procedural Macro Architecture**: Using proc-macros for JSON-RPC suggests focus on ergonomic, type-safe RPC interfaces with compile-time validation
2. **Workspace-First**: Structure anticipates multiple related crates sharing dependencies
3. **Agent Stack Focus**: Repository name indicates this is part of a larger agent system, with JSON-RPC as the communication protocol

### Integration Points
- **MCP (Model Context Protocol)**: Configured with Context7 for enhanced documentation access during development
- **Potential JS/TS Integration**: `.gitignore` includes `node_modules/`, suggesting possible JavaScript/TypeScript components

### Development Notes
- Edition 2024 is used (cutting edge Rust)
- The project is in early stages with foundational structure in place
- When implementing the JSON-RPC macro, typical dependencies will include `syn`, `quote`, and `proc-macro2`