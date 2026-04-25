//! Internal test helpers shared across the rust-agent-stack workspace.
//!
//! This crate is `publish = false` and intended only as a `dev-dependency` for
//! integration tests and benches. It exists to avoid duplicating mock auth
//! providers and server-spawn boilerplate across crates.

mod auth;
mod server;

pub use auth::{MockAuthProvider, mock_user};
pub use server::{spawn_http, spawn_tcp};
