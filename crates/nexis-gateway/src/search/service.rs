//! Search service implementation

use async_trait::async_trait;
use nexis_runtime::{EmbeddingProvider, EmbeddingRequest};
use nexis_vector::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Search request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Search query text
    pub query: String,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Minimum similarity score (0.0 to 1.0)
    pub min_score: Option<f32>,
    /// Filter to specific room
    pub room_id: Option<Uuid>,
    /// Include full content in results
    pub include_content: Option<bool>,
}

impl SearchRequest {
    /// Create a new search request
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            limit: None,
            min_score: None,
            room_id: None,
            include_content: None,
        }
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set minimum score threshold
    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = Some(score);
        self
    }

    /// Filter to specific room
    pub fn in_room(mut self, room_id: Uuid) -> Self {
        self.room_id = Some(room_id);
        self
    }
}

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// Document ID
    pub id: Uuid,
    /// Similarity score
    pub score: f32,
    /// Document content (if included)
    pub content: Option<String>,
    /// Room ID
    pub room_id: Option<Uuid>,
    /// Custom metadata
    pub metadata: serde_json::Value,
}

impl From<nexis_vector::SearchResult> for SearchResultItem {
    fn from(result: nexis_vector::SearchResult) -> Self {
        Self {
            id: result.document.id,
            score: result.score,
            content: Some(result.document.content),
            room_id: result.document.metadata.room_id,
            metadata: result.document.metadata.to_json(),
        }
    }
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Original query
    pub query: String,
    /// Search results
    pub results: Vec<SearchResultItem>,
    /// Total results found
    pub total: usize,
    /// Whether results were truncated
    pub truncated: bool,
}

impl SearchResponse {
    /// Create a new search response
    pub fn new(query: String, results: Vec<SearchResultItem>) -> Self {
        let total = results.len();
        Self {
            query,
            results,
            total,
            truncated: false,
        }
    }

    /// Mark response as truncated
    pub fn with_truncated(mut self) -> Self {
        self.truncated = true;
        self
    }
}

/// Search service trait
#[async_trait]
pub trait SearchService: Send + Sync {
    /// Perform semantic search
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse, SearchError>;

    /// Search within a specific room
    async fn search_in_room(
        &self,
        query: &str,
        room_id: Uuid,
        limit: usize,
    ) -> Result<SearchResponse, SearchError>;
}

/// Search error type
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Embedding generation failed: {0}")]
    EmbeddingError(String),

    #[error("Vector search failed: {0}")]
    VectorError(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

/// Semantic search service implementation
pub struct SemanticSearchService {
    vector_store: Arc<dyn VectorStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    default_limit: usize,
}

impl SemanticSearchService {
    /// Create a new semantic search service
    pub fn new(
        vector_store: Arc<dyn VectorStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            vector_store,
            embedding_provider,
            default_limit: 10,
        }
    }

    /// Set default result limit
    pub fn with_default_limit(mut self, limit: usize) -> Self {
        self.default_limit = limit;
        self
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, SearchError> {
        let req = EmbeddingRequest::new(text);
        let response = self
            .embedding_provider
            .embed(req)
            .await
            .map_err(|e| SearchError::EmbeddingError(e.to_string()))?;
        Ok(response.embedding)
    }
}

#[async_trait]
impl SearchService for SemanticSearchService {
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse, SearchError> {
        debug!("Searching for: {}", request.query);

        if request.query.trim().is_empty() {
            return Err(SearchError::InvalidQuery("Query cannot be empty".to_string()));
        }

        let embedding = self.generate_embedding(&request.query).await?;
        let query_vector = Vector::new(embedding);

        let limit = request.limit.unwrap_or(self.default_limit);
        let mut search_query = SearchQuery::new(query_vector).with_limit(limit);

        if let Some(min_score) = request.min_score {
            search_query = search_query.with_min_score(min_score);
        }

        if let Some(room_id) = request.room_id {
            search_query = search_query.with_filter(SearchFilter::new().with_room(room_id));
        }

        if !request.include_content.unwrap_or(true) {
            search_query = search_query.without_content();
        }

        let results = self
            .vector_store
            .search(search_query)
            .await
            .map_err(|e| SearchError::VectorError(e.to_string()))?;

        let items: Vec<SearchResultItem> = results.into_iter().map(SearchResultItem::from).collect();

        let truncated = items.len() >= limit;
        let mut response = SearchResponse::new(request.query, items);
        if truncated {
            response = response.with_truncated();
        }

        Ok(response)
    }

    async fn search_in_room(
        &self,
        query: &str,
        room_id: Uuid,
        limit: usize,
    ) -> Result<SearchResponse, SearchError> {
        let request = SearchRequest::new(query)
            .with_limit(limit)
            .in_room(room_id);
        self.search(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexis_runtime::MockEmbeddingProvider;
    use nexis_vector::InMemoryVectorStore;

    fn create_test_service() -> SemanticSearchService {
        let store = Arc::new(InMemoryVectorStore::new(128));
        let embedding = Arc::new(MockEmbeddingProvider::new(128));
        SemanticSearchService::new(store, embedding)
    }

    #[test]
    fn search_request_builder() {
        let room_id = Uuid::new_v4();
        let req = SearchRequest::new("test query")
            .with_limit(5)
            .with_min_score(0.5)
            .in_room(room_id);

        assert_eq!(req.query, "test query");
        assert_eq!(req.limit, Some(5));
        assert_eq!(req.min_score, Some(0.5));
        assert_eq!(req.room_id, Some(room_id));
    }

    #[tokio::test]
    async fn search_returns_empty_for_empty_index() {
        let service = create_test_service();
        let request = SearchRequest::new("test").with_limit(10);

        let response = service.search(request).await.unwrap();
        assert_eq!(response.total, 0);
        assert!(response.results.is_empty());
    }

    #[tokio::test]
    async fn search_rejects_empty_query() {
        let service = create_test_service();
        let request = SearchRequest::new("");

        let result = service.search(request).await;
        assert!(matches!(result, Err(SearchError::InvalidQuery(_))));
    }

    #[tokio::test]
    async fn search_in_room_uses_room_filter() {
        let service = create_test_service();
        let room_id = Uuid::new_v4();

        let response = service.search_in_room("test", room_id, 10).await.unwrap();
        assert_eq!(response.total, 0);
    }
}
