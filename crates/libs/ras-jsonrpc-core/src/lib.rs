//! Core authentication and authorization traits for JSON-RPC services.
//!
//! This crate provides the authentication and authorization traits used by the
//! `ras-jsonrpc-macro` procedural macro to generate type-safe JSON-RPC services
//! with axum integration.

// Re-export authentication types from ras-auth-core
pub use ras_auth_core::*;

// Re-export JSON-RPC types for convenience
pub use ras_jsonrpc_types::*;
