//! Nexis Gateway - Control Plane
//!
//! This crate implements the Control Plane for Nexis, handling:
//! - WebSocket connections
//! - Message routing
//! - Authentication and authorization
//! - Connection management
//! - Message indexing and semantic search
//! - Metrics and monitoring

pub mod auth;
pub mod connection;
pub mod db;
pub mod indexing;
pub mod metrics;
pub mod router;
pub mod search;
pub mod server;

#[allow(unused_imports)]
pub use auth::{AuthError, AuthenticatedUser, Claims, JwtConfig};
pub use indexing::{IndexingService, MessageIndexer};
pub use metrics::{export as export_metrics, init_metrics};
pub use router::build_routes;
pub use search::{SearchRequest, SearchResponse, SearchService, SemanticSearchService};

/// Gateway version
pub const GATEWAY_VERSION: &str = env!("CARGO_PKG_VERSION");
