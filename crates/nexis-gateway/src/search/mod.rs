//! Semantic search service for gateway
//!
//! This module provides:
//! - Semantic search across messages
//! - Room-scoped search
//! - Search result ranking and filtering

mod service;

pub use service::{SearchError, SearchRequest, SearchResponse, SearchService, SemanticSearchService};
