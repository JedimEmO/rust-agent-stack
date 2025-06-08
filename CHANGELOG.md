# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added - 2025-01-08
- OpenRPC document generation support for jsonrpc_service macro
  - Added optional `openrpc` field to macro invocation for per-service control
  - Supports both default path (`target/openrpc/{service_name}.json`) and custom output paths
  - Generates complete JSON Schema definitions using schemars crate for all request/response types
  - Includes authentication metadata with OpenRPC extensions (`x-authentication`, `x-permissions`)
  - Added comprehensive test coverage and examples demonstrating all features
  - Updated CLAUDE.md documentation with usage examples and requirements
  - Requires types to implement `schemars::JsonSchema` trait when OpenRPC generation is enabled

### Added - 2025-01-07
- Sprint retrospective implementation with project guidelines optimization
  - Streamlined CLAUDE.md documentation from verbose descriptions to concise guidelines
  - Added testing guidelines based on sprint observations (security-first, end-to-end testing)
  - Enhanced orchestrate command with key execution principles to prevent observed mistakes
  - Archived sprint-1 retrospective notes to scraim/retroed/ for historical tracking
  
- Added raitro command for automated sprint retrospectives
  - Command analyzes sprint observations and optimizes project guidelines
  - Provides framework for continuous improvement of development processes

### Fixed - 2025-01-07
- Fixed JSON-RPC macro routing issue causing 404 errors when accessing service endpoints
  - Macro now properly uses the base_url parameter instead of hardcoding "/" routes
  - Services created with custom paths (e.g., "/rpc") now work correctly when nested in routers
  - This resolves 404 errors in the Google OAuth2 example and other JSON-RPC services

- Fixed Axum router nesting syntax in Google OAuth2 example
  - Corrected router nesting from incorrect .merge() syntax to proper .nest() method
  - API endpoints now correctly accessible at /api/rpc instead of returning 404 errors

- Simplified Google OAuth2 example environment configuration template
  - Streamlined .env.example with cleaner formatting and reduced verbosity
  - Removed redundant comments and example credentials that could cause confusion
  - Improved clarity of required vs optional configuration parameters

- Fixed Google OAuth2 field compatibility issue preventing successful authentication callbacks
  - Added serde field alias to support both "sub" (OpenID Connect/v2/v3) and "id" (Google v1) user identifier fields
  - Updated Google OAuth example to use v3 userinfo endpoint for better feature support
  - Maintains backward compatibility with existing OAuth2 provider configurations
  - Added comprehensive tests for both field formats and additional claims handling

### Added - 2025-01-07
- Complete OAuth2 provider implementation with Google OAuth2 support and comprehensive security features
  - OAuth2Client with PKCE (Proof Key for Code Exchange) support for enhanced security
  - In-memory state store with automatic expiration and cleanup mechanisms
  - Complete authorization flow handling including code exchange and user info retrieval
  - Custom user info field mapping for flexible OAuth2 provider integration
  - Comprehensive error handling with OAuth2-specific error types and detailed context
  - Full test suite covering PKCE generation, authorization URLs, state management, and security scenarios
  - Production-ready implementation with proper HTTP timeouts and robust error recovery
- Enhanced JwtAuthProvider with Clone trait for improved service compatibility and architecture flexibility

### Added - 2025-01-07
- Google OAuth2 full-stack example application demonstrating complete authentication infrastructure
  - Interactive HTML/JS frontend with modern responsive design and real-time OAuth2 flow visualization
  - Complete Rust backend integration using Axum server with JSON-RPC API endpoints
  - Sophisticated permission system with role-based access control based on email domains and user attributes
  - Six different API endpoints showcasing permission-based access (user info, documents, admin, system status, beta features)
  - Production-ready OAuth2 flow with PKCE, state validation, JWT session management, and comprehensive error handling
  - Interactive API documentation with built-in testing capabilities and JWT token management
  - Comprehensive test suite covering permission logic and service compilation validation
  - Complete setup documentation with Google Cloud Console integration instructions

### Security - 2025-01-07
- Enhanced environment security with improved .gitignore patterns for secrets and credentials
  - Added comprehensive exclusion patterns for .env files, secrets directories, and OAuth2 credentials
  - Prevents accidental commitment of sensitive configuration data to version control
  - Includes protection for production, staging, and local environment configurations

### Documentation - 2025-01-07
- Updated CLAUDE.md with comprehensive Google OAuth2 example documentation and usage instructions
  - Added quick start guide with Google Cloud Console setup steps and environment configuration
  - Documented sophisticated permission system with role-based access control examples
  - Comprehensive API endpoint documentation with permission requirements and functionality descriptions
  - Added oauth2 provider status update from stub to full production-ready implementation
  - Enhanced development commands with example application execution instructions
  - Added Common Pitfalls section documenting Axum router nesting syntax issues
- Updated sprint reflection documentation with Google OAuth2 full-stack implementation learnings and coordination insights
  - Added reflection on OAuth2 example routing fix process and systematic debugging approach
  - Documented lessons learned about testing end-to-end flows and examining generated code

### Security - 2025-01-07
- Enhanced authentication security in rust-identity-local with comprehensive attack vector protection
  - Fixed username enumeration vulnerability - consistent errors for non-existent users and wrong passwords
  - Implemented timing attack resistance using constant-time authentication with real Argon2 dummy hash
  - Added robust input validation for malformed payloads, empty credentials, and special characters
  - Enhanced concurrent authentication safety and brute force protection
  - Comprehensive security test suite covering 11 attack vectors including password spraying and timing analysis
- Updated authentication architecture documentation with detailed security measures
- Added security considerations and attack vector protection guidelines to development documentation

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

### Fixed - 2025-01-07
- Resolved unused variable warning in JSON-RPC macro usage example

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