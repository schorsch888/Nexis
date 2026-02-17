//! Nexis Gateway - Control Plane
//!
//! This crate implements the Control Plane for Nexis, handling:
//! - WebSocket connections
//! - Message routing
//! - Authentication and authorization
//! - Connection management

pub mod auth;
pub mod connection;
pub mod router;
pub mod server;

#[allow(unused_imports)] // Re-exports for external use
pub use auth::{AuthError, AuthenticatedUser, Claims, JwtConfig};
pub use router::build_routes;

/// Gateway version
pub const GATEWAY_VERSION: &str = env!("CARGO_PKG_VERSION");
