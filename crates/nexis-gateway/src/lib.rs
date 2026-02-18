//! Nexis Gateway - Control Plane
//!
//! This crate implements the Control Plane for Nexis, handling:
//! - WebSocket connections
//! - Message routing
//! - Authentication and authorization
//! - Connection management
//! - Metrics and monitoring

pub mod auth;
pub mod connection;
pub mod db;
pub mod metrics;
pub mod router;
pub mod server;

#[allow(unused_imports)] // Re-exports for external use
pub use auth::{AuthError, AuthenticatedUser, Claims, JwtConfig};
pub use metrics::{export as export_metrics, init_metrics};
pub use router::build_routes;

/// Gateway version
pub const GATEWAY_VERSION: &str = env!("CARGO_PKG_VERSION");
