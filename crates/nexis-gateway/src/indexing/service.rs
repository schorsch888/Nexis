//! Indexing service implementation

use async_trait::async_trait;
use nexis_runtime::{EmbeddingProvider, EmbeddingRequest};
use nexis_vector::prelude::*;
use nexis_vector::DocumentMetadata;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::retry::{with_retry, RetryConfig};

/// Indexing service for processing messages into vector storage
#[async_trait]
pub trait IndexingService: Send + Sync {
    /// Index a message for semantic search
    async fn index_message(&self, message: &str, room_id: Uuid, metadata: serde_json::Value) -> IndexingResult<Uuid>;

    /// Search for similar messages
    async fn search(&self, query: &str, limit: usize) -> IndexingResult<Vec<SearchResult>>;

    /// Search within a specific room
    async fn search_in_room(&self, query: &str, room_id: Uuid, limit: usize) -> IndexingResult<Vec<SearchResult>>;
}

/// Message indexer configuration
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Vector dimension
    pub dimension: usize,
    /// Room ID for indexing
    pub default_room_id: Option<Uuid>,
    /// Retry configuration for embedding calls
    pub retry_config: RetryConfig,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            dimension: 1536,
            default_room_id: None,
            retry_config: RetryConfig::default(),
        }
    }
}

/// Message indexer that combines embedding and vector storage
pub struct MessageIndexer {
    vector_store: Arc<dyn VectorStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    config: IndexerConfig,
}

impl MessageIndexer {
    /// Create a new message indexer
    pub fn new(
        vector_store: Arc<dyn VectorStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        config: IndexerConfig,
    ) -> Self {
        Self { vector_store, embedding_provider, config }
    }

    /// Create with default configuration
    pub fn with_defaults(
        vector_store: Arc<dyn VectorStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self::new(vector_store, embedding_provider, IndexerConfig::default())
    }

    async fn generate_embedding(&self, text: &str) -> IndexingResult<Vec<f32>> {
        let text = text.to_string();
        let embedding = with_retry(
            || {
                let text = text.clone();
                let provider = self.embedding_provider.clone();
                async move {
                    let req = EmbeddingRequest::new(&text);
                    provider.embed(req).await
                }
            },
            &self.config.retry_config,
        )
        .await
        .map_err(|e| IndexingError::EmbeddingError(e.to_string()))?;

        Ok(embedding.embedding)
    }
}

#[async_trait]
impl IndexingService for MessageIndexer {
    async fn index_message(&self, message: &str, room_id: Uuid, metadata: serde_json::Value) -> IndexingResult<Uuid> {
        debug!("Indexing message for room: {}", room_id);

        let embedding = self.generate_embedding(message).await?;
        let vector = Vector::new(embedding);

        let metadata = DocumentMetadata::new()
            .with_room(room_id)
            .with_extra("custom", metadata);

        let doc = Document::new(vector, message.to_string(), metadata);

        self.vector_store.upsert(doc).await.map_err(|e| IndexingError::StorageError(e.to_string()))
    }

    async fn search(&self, query: &str, limit: usize) -> IndexingResult<Vec<SearchResult>> {
        debug!("Searching for: {}", query);

        let embedding = self.generate_embedding(query).await?;
        let query_vector = Vector::new(embedding);
        let search_query = SearchQuery::new(query_vector).with_limit(limit);

        self.vector_store.search(search_query).await.map_err(|e| IndexingError::StorageError(e.to_string()))
    }

    async fn search_in_room(&self, query: &str, room_id: Uuid, limit: usize) -> IndexingResult<Vec<SearchResult>> {
        debug!("Searching in room {} for: {}", room_id, query);

        let embedding = self.generate_embedding(query).await?;
        let query_vector = Vector::new(embedding);
        let search_query = SearchQuery::new(query_vector)
            .with_limit(limit)
            .with_filter(SearchFilter::new().with_room(room_id));

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
    use nexis_runtime::MockEmbeddingProvider;
    use nexis_vector::InMemoryVectorStore;

    #[tokio::test]
    async fn test_index_message() {
        let store = Arc::new(InMemoryVectorStore::new(1536));
        let embedding = Arc::new(MockEmbeddingProvider::new(1536));
        let indexer = MessageIndexer::with_defaults(store, embedding);

        let room_id = Uuid::new_v4();
        let metadata = serde_json::json!({"sender": "test"});

        let result = indexer.index_message("Hello world", room_id, metadata).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search() {
        let store = Arc::new(InMemoryVectorStore::new(1536));
        let embedding = Arc::new(MockEmbeddingProvider::new(1536));
        let indexer = MessageIndexer::with_defaults(store, embedding);

        let room_id = Uuid::new_v4();
        indexer.index_message("Test message", room_id, serde_json::json!({})).await.unwrap();

        let results = indexer.search("Test", 10).await;
        assert!(results.is_ok());
    }

    #[tokio::test]
    async fn test_search_in_room() {
        let store = Arc::new(InMemoryVectorStore::new(1536));
        let embedding = Arc::new(MockEmbeddingProvider::new(1536));
        let indexer = MessageIndexer::with_defaults(store, embedding);

        let room_id = Uuid::new_v4();
        indexer.index_message("Test message", room_id, serde_json::json!({})).await.unwrap();

        let results = indexer.search_in_room("Test", room_id, 10).await;
        assert!(results.is_ok());
    }
}
