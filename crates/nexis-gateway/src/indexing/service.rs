//! Indexing service implementation

use async_trait::async_trait;
use nexis_vector::prelude::*;
use nexis_vector::DocumentMetadata;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Indexing service for processing messages into vector storage
#[async_trait]
pub trait IndexingService: Send + Sync {
    /// Index a message for semantic search
    async fn index_message(&self, message: &str, room_id: Uuid, metadata: serde_json::Value) -> IndexingResult<Uuid>;

    /// Search for similar messages
    async fn search(&self, query: &str, limit: usize) -> IndexingResult<Vec<SearchResult>>;
}

/// Message indexer configuration
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Vector dimension
    pub dimension: usize,
    /// Room ID for indexing
    pub default_room_id: Option<Uuid>,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            dimension: 1536,
            default_room_id: None,
        }
    }
}

/// Message indexer that combines embedding and vector storage
pub struct MessageIndexer {
    vector_store: Arc<dyn VectorStore>,
    config: IndexerConfig,
}

impl MessageIndexer {
    /// Create a new message indexer
    pub fn new(vector_store: Arc<dyn VectorStore>, config: IndexerConfig) -> Self {
        Self { vector_store, config }
    }

    /// Create with default configuration
    pub fn with_defaults(vector_store: Arc<dyn VectorStore>) -> Self {
        Self::new(vector_store, IndexerConfig::default())
    }
}

#[async_trait]
impl IndexingService for MessageIndexer {
    async fn index_message(&self, message: &str, room_id: Uuid, metadata: serde_json::Value) -> IndexingResult<Uuid> {
        debug!("Indexing message for room: {}", room_id);

        // Create a placeholder vector (in real implementation, call embedding provider)
        let vector = Vector::new(vec![0.0; self.config.dimension]);

        let metadata = DocumentMetadata::new()
            .with_room(room_id)
            .with_extra("custom", metadata);

        let doc = Document::new(vector, message.to_string(), metadata);

        self.vector_store.upsert(doc).await.map_err(|e| IndexingError::StorageError(e.to_string()))
    }

    async fn search(&self, query: &str, limit: usize) -> IndexingResult<Vec<SearchResult>> {
        debug!("Searching for: {}", query);

        // Create a placeholder query vector
        let query_vector = Vector::new(vec![0.0; self.config.dimension]);
        let search_query = SearchQuery::new(query_vector).with_limit(limit);

        self.vector_store.search(search_query).await.map_err(|e| IndexingError::StorageError(e.to_string()))
    }
}

// Error types

/// Indexing error type
#[derive(Debug, thiserror::Error)]
pub enum IndexingError {
    #[error("Embedding generation failed: {0}")]
    EmbeddingError(String),

    #[error("Vector storage error: {0}")]
    StorageError(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}

/// Indexing result type
pub type IndexingResult<T> = Result<T, IndexingError>;

#[cfg(test)]
mod tests {
    use super::*;
    use nexis_vector::InMemoryVectorStore;

    #[tokio::test]
    async fn test_index_message() {
        let store = Arc::new(InMemoryVectorStore::new(1536));
        let indexer = MessageIndexer::with_defaults(store);

        let room_id = Uuid::new_v4();
        let metadata = serde_json::json!({"sender": "test"});

        let result = indexer.index_message("Hello world", room_id, metadata).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search() {
        let store = Arc::new(InMemoryVectorStore::new(1536));
        let indexer = MessageIndexer::with_defaults(store);

        let room_id = Uuid::new_v4();
        indexer.index_message("Test message", room_id, serde_json::json!({})).await.unwrap();

        let results = indexer.search("Test", 10).await;
        assert!(results.is_ok());
    }
}
