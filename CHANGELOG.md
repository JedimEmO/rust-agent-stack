# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added - 2025-01-07
- Complete JSON-RPC library ecosystem with three core crates
  - rust-jsonrpc-types: Pure JSON-RPC 2.0 protocol types and utilities
  - rust-jsonrpc-core: Authentication and authorization framework with AuthProvider trait
  - rust-jsonrpc-macro: Procedural macro for generating type-safe RPC interfaces with axum integration
- Comprehensive test suite and integration tests for macro functionality
- Workspace-level dependency management with shared crate versions
- Example applications demonstrating JSON-RPC service implementation
  - basic-service: Complete working example with authentication and multiple endpoints
  - Usage examples showing macro-generated service builders

### Added - 2025-01-06
- Initial project setup with Cargo workspace structure
- Created rust-jsonrpc-macro procedural macro crate foundation
- Added .gitignore for Rust and IDE artifacts
- Configured MCP integration with Context7 for enhanced documentation
- Added CLAUDE.md for AI-assisted development guidance