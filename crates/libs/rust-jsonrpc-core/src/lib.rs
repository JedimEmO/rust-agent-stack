//! Core authentication and authorization traits for JSON-RPC services.
//!
//! This crate provides the authentication and authorization traits used by the
//! `rust-jsonrpc-macro` procedural macro to generate type-safe JSON-RPC services
//! with axum integration.

// Re-export authentication types from rust-auth-core
pub use rust_auth_core::*;

// Re-export JSON-RPC types for convenience
pub use rust_jsonrpc_types::*;
