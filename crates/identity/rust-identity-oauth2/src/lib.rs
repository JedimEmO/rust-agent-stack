//! OAuth2 identity provider implementation with PKCE support.
//!
//! This crate provides a generic OAuth2 client that supports the Authorization Code flow
//! with PKCE (Proof Key for Code Exchange) for enhanced security. It integrates with the
//! rust-identity-core traits to provide OAuth2-based authentication.

mod client;
mod config;
mod error;
mod provider;
mod state;
mod types;

#[cfg(test)]
mod tests;

pub use client::{OAuth2Client, PkceChallenge};
pub use config::{OAuth2Config, OAuth2ProviderConfig};
pub use error::{OAuth2Error, OAuth2Result};
pub use provider::{OAuth2AuthPayload, OAuth2Provider, OAuth2Response};
pub use state::{InMemoryStateStore, OAuth2State, OAuth2StateStore};
pub use types::{AuthorizationRequest, AuthorizationResponse, TokenResponse, UserInfoResponse};

// Re-export common types for convenience
pub use rust_identity_core::{IdentityProvider, VerifiedIdentity};
