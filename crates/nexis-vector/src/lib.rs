//! Nexis Vector Storage - Vector store abstraction and semantic search
//!
//! This crate provides:
//! - `VectorStore` trait for abstracting vector storage backends
//! - In-memory vector store for testing
//! - Qdrant integration (optional, feature-gated)
//! - Semantic search capabilities

pub mod error;
pub mod store;
pub mod types;

pub use error::{VectorError, VectorResult};
pub use store::{InMemoryVectorStore, VectorStore};
pub use types::{Document, SearchQuery, SearchResult, Vector};

/// Prelude for common imports
pub mod prelude {
    pub use crate::error::{VectorError, VectorResult};
    pub use crate::store::VectorStore;
    pub use crate::types::{Document, SearchQuery, SearchResult, Vector};
}
