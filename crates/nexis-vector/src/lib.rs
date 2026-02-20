//! Nexis Vector Storage - Vector store abstraction and semantic search
//!
//! This crate provides:
//! - `VectorStore` trait for abstracting vector storage backends
//! - In-memory vector store for testing
//! - Qdrant integration (optional, feature-gated)
//! - Semantic search capabilities
//!
//! # Example
//!
//! ```rust,no_run
//! use nexis_vector::{InMemoryVectorStore, VectorStore, Document, Vector, DocumentMetadata, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() {
//!     let store = InMemoryVectorStore::new(1536);
//!     
//!     let doc = Document::new(
//!         Vector::new(vec![0.1; 1536]),
//!         "Hello world".to_string(),
//!         DocumentMetadata::new(),
//!     );
//!     
//!     let id = store.upsert(doc).await.unwrap();
//!     
//!     let query = SearchQuery::new(Vector::new(vec![0.1; 1536]))
//!         .with_limit(10);
//!     
//!     let results = store.search(query).await.unwrap();
//! }
//! ```

pub mod error;
pub mod store;
pub mod types;

#[cfg(feature = "qdrant")]
pub mod qdrant;

pub use error::{VectorError, VectorResult};
pub use store::{InMemoryVectorStore, VectorStore};
pub use types::{
    BatchResult, Document, DocumentMetadata, SearchFilter, SearchQuery, SearchResult, TimeRange,
    Vector,
};

#[cfg(feature = "qdrant")]
pub use qdrant::{QdrantConfig, QdrantVectorStore};

/// Prelude for common imports
pub mod prelude {
    pub use crate::error::{VectorError, VectorResult};
    pub use crate::store::VectorStore;
    pub use crate::types::{
        BatchResult, Document, DocumentMetadata, SearchFilter, SearchQuery, SearchResult, Vector,
    };

    #[cfg(feature = "qdrant")]
    pub use crate::qdrant::{QdrantConfig, QdrantVectorStore};
}
