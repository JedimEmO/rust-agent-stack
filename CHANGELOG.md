# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Changed - 2025-01-14
- Removed unused MCP server configurations (language-server, human-in-the-loop) from .mcp.json

### Fixed - 2025-01-14
- Updated minimum password length in chat server config examples to match 8-character validation requirement

### Refactored - 2025-01-14
- Simplified identity provider setup in bidirectional chat server
  - Removed unnecessary Arc wrapper for initial identity provider
  - Created separate registration provider instance sharing same user data
  - Improved code clarity while maintaining same functionality

### Added - 2025-01-14
- Bidirectional chat terminal client foundation (Sprint 2 Day 1)
  - Modular architecture with separate ui, client, auth, and config modules
  - Complete ratatui-based terminal UI with message area, user list, and input field
  - Placeholder implementations for WebSocket client integration
  - Configuration system supporting environment variables and TOML files
  - JWT token management infrastructure for authentication

### Updated - 2025-01-14
- Simplified CLAUDE.md build commands to use generic examples instead of listing all crates
- Added bidirectional chat client architecture details to documentation
  - Terminal UI layout and components
  - State management and WebSocket integration
  - Authentication and configuration details
- Updated TASK.md to mark completed Sprint 2 terminal client implementation tasks
- Archived Sprint 3 retrospective to scraim/retroed/ folder
  - Documented successful completion of bidirectional chat server and client foundation

### Added - 2025-01-13
- Comprehensive configuration system for bidirectional chat server
  - Flexible configuration supporting environment variables and TOML files
  - Server, auth, chat, logging, admin, and rate limit settings
  - Legacy environment variable support for backward compatibility
  - Configuration validation with helpful error messages
  - Example config file and test utility for validation

- Structured logging with tracing for bidirectional chat server
  - Configurable log levels and formats (pretty, JSON, compact)
  - Structured logging with connection IDs, user info, and room details
  - Debug/trace logging for detailed troubleshooting
  - Configuration via RUST_LOG environment variable or config file

- Comprehensive integration tests for bidirectional chat server
  - Server integration tests covering startup, config, auth, and persistence
  - WebSocket tests for connection lifecycle and authentication
  - Concurrent user scenarios and permission handling tests
  - Port management for parallel test execution
  - Complete test coverage of all server features

- Enhanced persistence layer with structured logging
  - Added tracing to all file operations and state management
  - Error context with detailed failure messages
  - Parse error tracking when loading corrupted messages
  - Operation metrics for state loading/saving

### Added - 2025-01-13
- Added ideate command for interactive brainstorming and execution planning
  - New .claude/commands/ideate.md facilitates collaborative idea development
  - Updated plan.md to emphasize brainstorming before work breakdown

- Bidirectional chat example demonstrating real-time WebSocket communication
  - Complete chat server with room management and message persistence
  - CLI client with register/login/chat commands for interactive sessions
  - JWT-based authentication with role-based permissions (user/admin)
  - Persistent chat history using JSON file storage
  - Type-safe bidirectional RPC using generated client/server code
  - Updated CLAUDE.md with bidirectional macro implementation notes

- User profile system with cat avatar customization
  - Added profile management endpoints (get_profile, update_profile)
  - Support for 10 cat breeds, 10 colors, and 8 expressions
  - Integrated profile persistence with existing state management
  - Profile creation during user registration

### Fixed - 2025-01-09
- Fixed bidirectional WebSocket channel management synchronization issue causing test failures
  - Extended ConnectionManager trait with add_connection_with_sender method for proper channel registration
  - Fixed WebSocket service to register actual message channels instead of creating dummy channels
  - Resolved "channel closed" errors and timeout issues in bidirectional communication tests
  - Enhanced DefaultConnectionManager to handle real channel registration via downcasting
  - All 22 bidirectional JSON-RPC tests now pass with proper connection management

### Added - 2025-01-09
- Enhanced bidirectional JSON-RPC macro with server-side client management capabilities
  - Service trait methods now receive client connection ID and connection manager reference 
  - Connection lifecycle hooks: on_client_connected, on_client_disconnected, on_client_authenticated
  - Typed client handles for direct server-to-client communication and connection management
  - Real-time broadcasting capabilities within service implementations
  - Full access to connection manager for advanced client tracking and messaging patterns

### Added - 2025-01-09
- Type-safe client generation for both JSON-RPC and REST services with comprehensive API coverage
  - Implemented builder pattern client APIs with reqwest for HTTP communication
  - Added feature flags (server/client) for optional dependency management and modular builds
  - Bearer token authentication support with get/set methods for secure API access
  - Timeout configuration for both default and per-request timeout handling
  - Cross-platform compatibility using reqwest for both x86 and WASM targets
  - Generated client methods match server API signatures exactly for type safety
  - Zero breaking changes with full backward compatibility for existing server-only code
  - Optional client dependencies (reqwest) only loaded when client feature enabled
  - Comprehensive test coverage for client generation and HTTP communication patterns

### Fixed - 2025-01-09
- Improved Bruno auth enum formatting for better code consistency
  - Fixed formatting of BrunoAuth enum to use consistent brace style
  - Enhanced readability with proper field alignment for Bearer and Basic auth types
  - Maintained proper code formatting standards throughout bruno.rs module

### Fixed - 2025-01-09
- Fixed OpenRPC schema generation to comply with JSON-RPC specification
  - Schema definitions now properly use components/schemas instead of $defs
  - Service-specific helper functions prevent naming conflicts in generated code
  - All schema references updated to use standard #/components/schemas/ format

### Added - 2025-01-09
- New OpenRPC-to-Bruno conversion tool for generating Bruno API collections from OpenRPC specifications
  - Complete CLI tool `openrpc-to-bruno` for converting OpenRPC 1.3.2 documents to Bruno collections
  - Supports authentication extraction with Bearer token configuration
  - Generates environment variables and collection metadata automatically
  - Comprehensive test suite with integration tests for conversion accuracy
  - Handles method parameter conversion with proper JSON schema validation
  - Bruno collection format support with proper .bru file generation
  - Command-line interface with configurable output directories and collection naming

### Refactored - 2025-01-09
- Restructured Google OAuth example into multi-crate architecture for better separation of concerns
  - Split into separate `api` and `server` crates with clean API boundary separation
  - API crate contains service definitions and OpenRPC generation logic
  - Server crate focuses on HTTP routing, authentication, and frontend serving
  - Build-time OpenRPC generation moved to build.rs for automatic documentation updates
  - Improved static file serving with relative paths for better deployment flexibility
  - Enhanced example structure provides clearer patterns for real-world applications

### Enhanced - 2025-01-09
- Updated workspace configuration and dependencies to support new tooling and improved development experience
  - Added clap workspace dependency for consistent CLI tooling across the project
  - Updated schemars to 1.0.0-alpha.20 for improved JSON Schema Draft 7 compatibility
  - Enhanced workspace member organization with tools and multi-crate example structure
  - Fixed import ordering in integration tests following Rust style guidelines
  - Improved Cargo.lock with new dependencies for CLI tools and testing infrastructure

### Fixed - 2025-01-09
- Fixed OpenRPC specification parsing to support extension fields and JSON Schema compatibility
  - Removed deny_unknown_fields restrictions from Method and Schema structs in openrpc-types crate
  - Added $schema field support to Schema struct for proper JSON Schema Draft 7 compatibility
  - Enables proper parsing of OpenRPC documents with x-authentication and x-permissions extensions
  - Bruno API collection generator now properly supports OpenRPC files with custom authentication metadata

### Enhanced - 2025-01-09
- Enhanced OpenRPC document generation functionality to actually generate files
  - Modified google-oauth-example to call OpenRPC generation functions during service creation
  - Added JsonSchema derives to all request/response types for proper schema generation
  - Created test infrastructure to verify end-to-end OpenRPC generation works correctly
  - OpenRPC documents now properly written to target/openrpc/ directory when enabled

### Fixed - 2025-01-09
- Fixed Bruno API collection JSON formatting to be properly indented and valid
  - Corrected JSON body indentation in .bru files to use proper 2-space indentation within body:json blocks
  - Generated Bruno collections are now properly formatted and compatible with Bruno API client
  - Resolves validation errors when importing generated collections into Bruno

### Documentation - 2025-01-09
- Added comprehensive OpenRPC generation documentation to ras-jsonrpc-macro README
  - Documented OpenRPC generation feature with complete usage examples and configuration options
  - Included requirements for JsonSchema trait implementation on request/response types
  - Added examples for both boolean and custom path OpenRPC generation configurations
  - Explained generated function signatures and integration patterns

### Enhanced - 2025-01-08
- Refactored permission system to support AND/OR logic groups for both REST and JSON-RPC macros
  - Changed permission syntax from flat array to nested groups with OR logic between groups and AND logic within groups
  - `WITH_PERMISSIONS(["admin", "moderator"])` now requires user to have both admin AND moderator permissions
  - `WITH_PERMISSIONS(["admin", "moderator"] | ["super_user"])` allows (admin AND moderator) OR super_user access
  - Supports multiple OR groups for complex permission combinations
  - Updated both REST and JSON-RPC macros simultaneously to ensure consistent behavior
  - Enhanced test coverage with new test cases demonstrating OR group functionality
  - Backward compatible syntax for existing single-group permissions
  - OpenAPI and OpenRPC documentation generation handles new permission structure correctly

### Fixed - 2025-01-08
- Fixed REST macro integration test failures with improved error handling and permission logic
  - Enhanced JSON error handling to return proper 400 status codes instead of 422 for invalid JSON requests
  - Fixed permission checking logic to use OR semantics (user needs ANY of the required permissions) instead of AND semantics
  - Improved macro-generated code to handle JSON parsing errors gracefully with appropriate HTTP status codes
  - Resolved test failures in `test_multiple_permissions_endpoints` and `test_invalid_requests`
  - Permission system now properly allows users with any of the listed permissions to access endpoints

### Fixed - 2025-01-08
- Fixed REST service example endpoint syntax for empty parameter methods
  - Corrected auth/logout and auth/me endpoint definitions to use proper empty parameter syntax
  - Updated handler signatures to match macro-generated function signatures for parameterless endpoints
  - Improved consistency with REST macro patterns for endpoints that don't require request bodies

### Fixed - 2025-01-08
- Fixed JSON-RPC macro parameter handling for unit type `()` parameters
  - Enhanced macro-generated code to properly handle methods with unit type parameters when no params are provided
  - Fixed parameter parsing to deserialize `None` parameters as `serde_json::Value::Null` for unit types instead of rejecting as invalid
  - Resolved test failures in `test_unauthorized_methods`, `test_authentication_required_methods`, `test_admin_permission_methods`, and `test_concurrent_requests`
  - Improved backward compatibility for JSON-RPC requests with missing or null parameters for void methods

### Added - 2025-01-08
- Comprehensive HTTP integration test suites for both JSON-RPC and REST macro crates
  - Complete JSON-RPC integration tests covering all authentication patterns (UNAUTHORIZED, WITH_PERMISSIONS with various levels)
  - Full REST API integration tests with CRUD operations, path parameters, and HTTP method validation
  - Real HTTP server testing using random port binding with tokio TcpListener for concurrent test execution
  - Authentication and authorization testing across all permission levels with JWT token validation
  - Security testing including timing attack resistance and proper error handling scenarios
  - Concurrent request testing validating thread safety and performance under load
  - OpenRPC and OpenAPI document generation testing ensuring specification compliance
  - Test infrastructure supporting both positive and negative scenarios with comprehensive error validation
  - Fixed unused import warnings in rust-identity-local during test infrastructure development

### Enhanced - 2025-01-08
- Added comprehensive testing dependencies for HTTP integration testing across macro crates
  - Added wiremock, reqwest, tower, hyper, rand, and futures to workspace dependencies for robust HTTP testing infrastructure
  - Enhanced rust-jsonrpc-macro and rust-rest-macro with testing dependencies for real server integration tests
  - Established foundation for comprehensive integration testing with random port binding and concurrent request handling
  - Dependencies support both JSON-RPC and REST API testing patterns with authentication validation

### Refactored - 2025-01-08
- Architectural refactoring to eliminate coupling between RPC and REST macro crates
  - Created new `rust-auth-core` crate as shared foundation for authentication types and traits
  - Moved `AuthProvider`, `AuthenticatedUser`, `AuthError`, and related types from `rust-jsonrpc-core` to `rust-auth-core`
  - Updated `rust-rest-macro` to depend on `rust-auth-core` instead of `rust-jsonrpc-core`, eliminating unwanted cross-dependencies
  - Updated `rust-identity-session` and other affected crates to use shared authentication types
  - Maintained full backward compatibility through re-exports in `rust-jsonrpc-core`
  - Enhanced codebase maintainability with clear separation of concerns between authentication logic and protocol-specific implementations
  - Improved workspace architecture enabling future protocol extensions (gRPC, etc.) without introducing coupling
  - Updated documentation and build commands to reflect new crate structure

### Fixed - 2025-01-08
- Fixed REST service example authentication provider sharing issue
  - Resolved authentication failures after user registration due to provider instance isolation
  - Implemented SharedUserProvider wrapper to ensure consistent provider state across service components
  - Fixed issue where LocalUserProvider instance used for registration differed from SessionService instance
  - Authentication now works correctly for both pre-configured test users (admin/admin123, user/user123) and newly registered users
  - Enhanced code organization with proper provider lifecycle management

### Fixed - 2025-01-08
- Fixed REST API documentation schema display for optional fields showing as empty objects
  - Enhanced OpenAPI schema generation to convert `"type": ["string", "null"]` format to `"type": "string", "nullable": true"` for better Swagger UI compatibility
  - Improved JavaScript schema processing in documentation UI to handle array type definitions (e.g., `["string", "null"]`)
  - Added recursive schema normalization for all nested objects and definitions
  - Optional fields like `email` and `display_name` now display as proper string input fields with meaningful examples
  - Both backend OpenAPI generation and frontend UI handling improved for comprehensive fix

### Enhanced - 2025-01-08
- Sprint retrospective update covering Static API Documentation Hosting & Explorer UI implementation
  - Documented strategic orchestration approach with successful role delegation (Architect → Backend Coder → UX Designer)
  - Noted seamless integration with existing rust-rest-macro patterns without breaking changes
  - Recognized custom API explorer UI success replacing generic Swagger UI with tailored features
  - Highlighted zero-overhead implementation design for optional features
  - Identified opportunity for smaller proof-of-concept approach in future complex implementations

### Added - 2025-01-08
- Static API documentation hosting with embedded explorer UI for REST services
  - Complete static file hosting support integrated into rust-rest-macro crate
  - Interactive API documentation with custom-built explorer UI replacing generic Swagger UI
  - Embedded static assets using rust-embed for zero-dependency deployment
  - JWT authentication integration directly in the explorer interface
  - Responsive documentation UI with multiple theme support (default theme included)
  - Automatic OpenAPI spec serving at configurable endpoints
  - Optional feature with zero overhead when disabled - no performance impact
  - Enhanced REST service example showcasing documentation hosting capabilities
  - Configurable documentation paths and themes via macro parameters

### Enhanced - 2025-01-08
- Sprint retrospective process with enhanced development guidelines based on observed patterns
  - Added Critical Development Rules section to CLAUDE.md based on sprint observation analysis
  - Five new rules: Test Early/Often, Specification First, Incremental Implementation, Macro Testing, End-to-End Validation
  - Enhanced Common Pitfalls with string type mismatches and move semantics guidance
  - Updated crate listings to include rust-rest-macro and build commands
  - Archived sprint-2 retrospective notes covering OpenRPC generation, registry setup, and REST macro implementation
  - Systematic approach to learning from development patterns and preventing recurring issues

### Enhanced - 2025-01-08
- REST service example now demonstrates complete local authentication integration with comprehensive security features
  - Full JWT-based authentication using rust-identity-local and rust-identity-session crates
  - Complete auth endpoints: user registration, login, logout, and user info retrieval
  - Role-based permission system with admin and user access levels (admin users inherit user permissions)
  - Two-phase authentication flow: LocalUserProvider for credential validation → SessionService for JWT issuance
  - Pre-configured test users (admin/admin123 with admin permissions, user/user123 with user permissions)
  - Environment-based configuration for JWT secrets, server host/port with secure defaults
  - Protected REST endpoints demonstrating permission-based access control in action
  - Comprehensive security implementation with Argon2 password hashing and session tracking

### Added - 2025-01-08
- REST macro crate implementation with comprehensive REST API generation capabilities
  - Complete rust-rest-macro procedural macro crate for type-safe REST endpoints with authentication integration
  - Supports all HTTP methods (GET, POST, PUT, DELETE, PATCH) with path parameters and request bodies
  - OpenAPI 3.0 document generation using schemars with configurable output paths
  - Permission-based access control with JWT authentication through AuthProvider integration
  - Generated service traits, builders, and axum router integration following JSON-RPC macro patterns
  - Example application (rest-service-example) demonstrating comprehensive REST service implementation
  - Full workspace integration with proper dependency management and testing infrastructure

### Added - 2025-01-08
- Kellnr registry configuration for local crate publishing
  - Configured kellnr as default registry in `.cargo/config.toml`
  - Registry URL set to `http://localhost:8000/api/v1/crates/`
  - Created comprehensive release command at `.claude/commands/kellnr-release.md`
  - Includes A-Z release process with dependency order management
  - All internal dependencies already properly configured with path + version

### Added - 2025-01-08
- Complete OpenRPC 1.3.2 specification types crate (openrpc-types) with full type safety and validation
  - Comprehensive implementation of all OpenRPC specification types with serde serialization support
  - Ergonomic builder patterns using bon crate for fluent API construction
  - Extensive validation system for OpenRPC documents, method names, error codes, and component references
  - JSON Schema Draft 7 support with schemars integration for automatic schema generation
  - 142 comprehensive unit tests covering all types, builders, validation rules, and serialization scenarios
  - Complete documentation with working examples and doctest validation
  - Full workspace integration following established dependency patterns

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