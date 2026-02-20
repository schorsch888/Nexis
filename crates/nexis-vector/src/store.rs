//! Vector store trait and implementations

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{VectorError, VectorResult};
use crate::types::{Document, SearchQuery, SearchResult};

/// Vector store abstraction
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Store a document
    async fn upsert(&self, document: Document) -> VectorResult<Uuid>;

    /// Get a document by ID
    async fn get(&self, id: Uuid) -> VectorResult<Document>;

    /// Delete a document
    async fn delete(&self, id: Uuid) -> VectorResult<()>;

    /// Search for similar documents
    async fn search(&self, query: SearchQuery) -> VectorResult<Vec<SearchResult>>;

    /// Get the dimension of vectors in this store
    fn dimension(&self) -> usize;
}

/// In-memory vector store for testing
pub struct InMemoryVectorStore {
    documents: Arc<RwLock<HashMap<Uuid, Document>>>,
    dimension: usize,
}

impl InMemoryVectorStore {
    /// Create a new in-memory store
    pub fn new(dimension: usize) -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            dimension,
        }
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&self, document: Document) -> VectorResult<Uuid> {
        if document.vector.dimensions != self.dimension {
            return Err(VectorError::InvalidDimension {
                expected: self.dimension,
                actual: document.vector.dimensions,
            });
        }
        let id = document.id;
        self.documents.write().await.insert(id, document);
        Ok(id)
    }

    async fn get(&self, id: Uuid) -> VectorResult<Document> {
        self.documents
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| VectorError::NotFound(id.to_string()))
    }

    async fn delete(&self, id: Uuid) -> VectorResult<()> {
        self.documents
            .write()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| VectorError::NotFound(id.to_string()))
    }

    async fn search(&self, query: SearchQuery) -> VectorResult<Vec<SearchResult>> {
        let docs = self.documents.read().await;
        let mut results: Vec<SearchResult> = docs
            .values()
            .map(|doc| {
                let score = query.vector.cosine_similarity(&doc.vector);
                SearchResult {
                    document: doc.clone(),
                    score,
                }
            })
            .filter(|r| query.min_score.map_or(true, |min| r.score >= min))
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(query.limit);

        Ok(results)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upsert_and_get() {
        let store = InMemoryVectorStore::new(3);
        let vector = Vector::new(vec![1.0, 0.0, 0.0]);
        let doc = Document::new(vector, "test".to_string(), serde_json::json!({}));

        let id = store.upsert(doc.clone()).await.unwrap();
        let retrieved = store.get(id).await.unwrap();

        assert_eq!(doc.content, retrieved.content);
    }

    #[tokio::test]
    async fn test_search() {
        let store = InMemoryVectorStore::new(3);

        // Store some documents
        let doc1 = Document::new(
            Vector::new(vec![1.0, 0.0, 0.0]),
            "first".to_string(),
            serde_json::json!({}),
        );
        let doc2 = Document::new(
            Vector::new(vec![0.9, 0.1, 0.0]),
            "second".to_string(),
            serde_json::json!({}),
        );
        let doc3 = Document::new(
            Vector::new(vec![0.0, 0.0, 1.0]),
            "third".to_string(),
            serde_json::json!({}),
        );

        store.upsert(doc1).await.unwrap();
        store.upsert(doc2).await.unwrap();
        store.upsert(doc3).await.unwrap();

        // Search
        let query = SearchQuery::new(Vector::new(vec![1.0, 0.0, 0.0])).with_limit(2);
        let results = store.search(query).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryVectorStore::new(3);
        let doc = Document::new(
            Vector::new(vec![1.0, 0.0, 0.0]),
            "test".to_string(),
            serde_json::json!({}),
        );

        let id = store.upsert(doc).await.unwrap();
        store.delete(id).await.unwrap();

        assert!(store.get(id).await.is_err());
    }
}
