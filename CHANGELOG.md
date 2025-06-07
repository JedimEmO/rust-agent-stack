# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added - 2025-01-07
- Identity management system with pluggable authentication providers
  - rust-identity-core: Core traits for IdentityProvider and UserPermissions with default implementations
  - rust-identity-local: Local username/password authentication with Argon2 password hashing
  - rust-identity-oauth2: OAuth2 provider framework (stub implementation for future completion)
  - rust-identity-session: JWT-based session management with configurable secrets and permission lookup
- Two-stage authentication flow: identity verification followed by JWT session creation
- Permission system with UserPermissions trait enabling flexible RBAC patterns
- JwtAuthProvider implementing AuthProvider trait for seamless JSON-RPC integration
- Comprehensive test suite covering authentication workflows and permission assignment
- Design documentation and architecture patterns for identity management
- Workspace configuration updates to include identity management crates

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
- Enhanced project documentation and development guidelines
  - Updated CLAUDE.md with comprehensive crate organization patterns
  - Added development workflow instructions and dependency management guidelines
  - Improved orchestration commands for better AI-assisted development
- Sprint reflection system for tracking development progress and learnings

### Added - 2025-01-06
- Initial project setup with Cargo workspace structure
- Created rust-jsonrpc-macro procedural macro crate foundation
- Added .gitignore for Rust and IDE artifacts
- Configured MCP integration with Context7 for enhanced documentation
- Added CLAUDE.md for AI-assisted development guidance