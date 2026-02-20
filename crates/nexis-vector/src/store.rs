//! Vector store trait and implementations

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{VectorError, VectorResult};
use crate::types::{BatchResult, Document, SearchQuery, SearchResult};

/// Vector store abstraction
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Store or update a document
    async fn upsert(&self, document: Document) -> VectorResult<Uuid>;

    /// Store or update multiple documents
    async fn upsert_batch(&self, documents: Vec<Document>) -> VectorResult<BatchResult>;

    /// Get a document by ID
    async fn get(&self, id: Uuid) -> VectorResult<Document>;

    /// Get multiple documents by ID
    async fn get_batch(&self, ids: Vec<Uuid>) -> VectorResult<Vec<Document>>;

    /// Delete a document by ID
    async fn delete(&self, id: Uuid) -> VectorResult<()>;

    /// Delete multiple documents by ID
    async fn delete_batch(&self, ids: Vec<Uuid>) -> VectorResult<BatchResult>;

    /// Search for similar documents
    async fn search(&self, query: SearchQuery) -> VectorResult<Vec<SearchResult>>;

    /// Count documents in the store
    async fn count(&self) -> VectorResult<usize>;

    /// Check if a document exists
    async fn exists(&self, id: Uuid) -> VectorResult<bool>;

    /// Get the dimension of vectors in this store
    fn dimension(&self) -> usize;

    /// Get the name of this store backend
    fn backend_name(&self) -> &'static str;
}

/// In-memory vector store for testing and development
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

    /// Clear all documents
    pub async fn clear(&self) {
        self.documents.write().await.clear();
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new(1536)
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&self, document: Document) -> VectorResult<Uuid> {
        if document.vector.dimensions != self.dimension {
            return Err(VectorError::invalid_dimension(
                self.dimension,
                document.vector.dimensions,
            ));
        }
        let id = document.id;
        self.documents.write().await.insert(id, document);
        Ok(id)
    }

    async fn upsert_batch(&self, documents: Vec<Document>) -> VectorResult<BatchResult> {
        let mut result = BatchResult::new();
        let mut docs = self.documents.write().await;

        for doc in documents {
            if doc.vector.dimensions != self.dimension {
                result.add_failure(
                    doc.id,
                    format!(
                        "Invalid dimension: expected {}, got {}",
                        self.dimension, doc.vector.dimensions
                    ),
                );
            } else {
                result.add_success(doc.id);
                docs.insert(doc.id, doc);
            }
        }

        Ok(result)
    }

    async fn get(&self, id: Uuid) -> VectorResult<Document> {
        self.documents
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| VectorError::not_found(id))
    }

    async fn get_batch(&self, ids: Vec<Uuid>) -> VectorResult<Vec<Document>> {
        let docs = self.documents.read().await;
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(doc) = docs.get(&id) {
                result.push(doc.clone());
            }
        }
        Ok(result)
    }

    async fn delete(&self, id: Uuid) -> VectorResult<()> {
        self.documents
            .write()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| VectorError::not_found(id))
    }

    async fn delete_batch(&self, ids: Vec<Uuid>) -> VectorResult<BatchResult> {
        let mut result = BatchResult::new();
        let mut docs = self.documents.write().await;

        for id in ids {
            if docs.remove(&id).is_some() {
                result.add_success(id);
            } else {
                result.add_failure(id, "Document not found".to_string());
            }
        }

        Ok(result)
    }

    async fn search(&self, query: SearchQuery) -> VectorResult<Vec<SearchResult>> {
        query.validate().map_err(VectorError::invalid_query)?;

        let docs = self.documents.read().await;
        let mut results: Vec<SearchResult> = docs
            .values()
            .filter(|doc| {
                query
                    .filter
                    .as_ref()
                    .is_none_or(|f| f.matches(doc))
            })
            .map(|doc| {
                let score = query.vector.cosine_similarity(&doc.vector);
                SearchResult::new(doc.clone(), score)
            })
            .filter(|r| query.min_score.is_none_or(|min| r.score >= min))
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if query.offset > 0 {
            results = results.into_iter().skip(query.offset).collect();
        }
        results.truncate(query.limit);

        Ok(results)
    }

    async fn count(&self) -> VectorResult<usize> {
        Ok(self.documents.read().await.len())
    }

    async fn exists(&self, id: Uuid) -> VectorResult<bool> {
        Ok(self.documents.read().await.contains_key(&id))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn backend_name(&self) -> &'static str {
        "in-memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DocumentMetadata, SearchFilter, Vector};
    use chrono::{Duration, Utc};

    fn create_test_doc(content: &str, vector_data: Vec<f32>) -> Document {
        Document::new(
            Vector::new(vector_data),
            content.to_string(),
            DocumentMetadata::new(),
        )
    }

    #[tokio::test]
    async fn test_upsert_and_get() {
        let store = InMemoryVectorStore::new(3);
        let doc = create_test_doc("test", vec![1.0, 0.0, 0.0]);

        let id = store.upsert(doc.clone()).await.unwrap();
        let retrieved = store.get(id).await.unwrap();

        assert_eq!(doc.content, retrieved.content);
        assert_eq!(doc.id, retrieved.id);
    }

    #[tokio::test]
    async fn test_upsert_batch() {
        let store = InMemoryVectorStore::new(3);
        let docs = vec![
            create_test_doc("doc1", vec![1.0, 0.0, 0.0]),
            create_test_doc("doc2", vec![0.0, 1.0, 0.0]),
        ];

        let result = store.upsert_batch(docs).await.unwrap();
        assert_eq!(result.succeeded.len(), 2);
        assert!(result.is_all_success());
    }

    #[tokio::test]
    async fn test_upsert_batch_with_invalid_dimension() {
        let store = InMemoryVectorStore::new(3);
        let docs = vec![
            create_test_doc("valid", vec![1.0, 0.0, 0.0]),
            Document::new(
                Vector::new(vec![1.0, 0.0]),
                "invalid".to_string(),
                DocumentMetadata::new(),
            ),
        ];

        let result = store.upsert_batch(docs).await.unwrap();
        assert_eq!(result.succeeded.len(), 1);
        assert_eq!(result.failed.len(), 1);
    }

    #[tokio::test]
    async fn test_get_batch() {
        let store = InMemoryVectorStore::new(3);
        let doc1 = create_test_doc("doc1", vec![1.0, 0.0, 0.0]);
        let doc2 = create_test_doc("doc2", vec![0.0, 1.0, 0.0]);

        let id1 = store.upsert(doc1).await.unwrap();
        let id2 = store.upsert(doc2).await.unwrap();

        let retrieved = store.get_batch(vec![id1, id2]).await.unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[tokio::test]
    async fn test_search_with_filter() {
        let store = InMemoryVectorStore::new(3);
        let room_id = Uuid::new_v4();

        let doc1 = Document::new(
            Vector::new(vec![1.0, 0.0, 0.0]),
            "first".to_string(),
            DocumentMetadata::new().with_room(room_id),
        );
        let doc2 = Document::new(
            Vector::new(vec![1.0, 0.1, 0.0]),
            "second".to_string(),
            DocumentMetadata::new(),
        );

        store.upsert(doc1).await.unwrap();
        store.upsert(doc2).await.unwrap();

        let filter = SearchFilter::new().with_room(room_id);
        let query = SearchQuery::new(Vector::new(vec![1.0, 0.0, 0.0])).with_filter(filter);

        let results = store.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.content, "first");
    }

    #[tokio::test]
    async fn test_search_with_min_score() {
        let store = InMemoryVectorStore::new(3);

        store.upsert(create_test_doc("doc1", vec![1.0, 0.0, 0.0])).await.unwrap();
        store.upsert(create_test_doc("doc2", vec![0.0, 0.0, 1.0])).await.unwrap();

        let query = SearchQuery::new(Vector::new(vec![1.0, 0.0, 0.0]))
            .with_min_score(0.9);

        let results = store.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.content, "doc1");
    }

    #[tokio::test]
    async fn test_search_with_time_range() {
        let store = InMemoryVectorStore::new(3);

        let now = Utc::now();
        let doc = Document::new(
            Vector::new(vec![1.0, 0.0, 0.0]),
            "timed".to_string(),
            DocumentMetadata::new(),
        );

        store.upsert(doc).await.unwrap();

        let filter = SearchFilter::new()
            .with_time_range(now - Duration::hours(1), now + Duration::hours(1));
        let query = SearchQuery::new(Vector::new(vec![1.0, 0.0, 0.0]))
            .with_filter(filter);

        let results = store.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_pagination() {
        let store = InMemoryVectorStore::new(3);

        for i in 0..5 {
            store
                .upsert(create_test_doc(&format!("doc{i}"), vec![1.0, 0.0, 0.0]))
                .await
                .unwrap();
        }

        let query = SearchQuery::new(Vector::new(vec![1.0, 0.0, 0.0]))
            .with_limit(2)
            .with_offset(2);

        let results = store.search(query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryVectorStore::new(3);
        let doc = create_test_doc("test", vec![1.0, 0.0, 0.0]);

        let id = store.upsert(doc).await.unwrap();
        store.delete(id).await.unwrap();

        assert!(store.get(id).await.is_err());
    }

    #[tokio::test]
    async fn test_delete_batch() {
        let store = InMemoryVectorStore::new(3);
        let doc1 = create_test_doc("doc1", vec![1.0, 0.0, 0.0]);
        let doc2 = create_test_doc("doc2", vec![0.0, 1.0, 0.0]);

        let id1 = store.upsert(doc1).await.unwrap();
        let id2 = store.upsert(doc2).await.unwrap();

        let result = store.delete_batch(vec![id1, id2]).await.unwrap();
        assert_eq!(result.succeeded.len(), 2);
        assert!(store.get(id1).await.is_err());
        assert!(store.get(id2).await.is_err());
    }

    #[tokio::test]
    async fn test_count_and_exists() {
        let store = InMemoryVectorStore::new(3);
        let doc = create_test_doc("test", vec![1.0, 0.0, 0.0]);

        assert_eq!(store.count().await.unwrap(), 0);

        let id = store.upsert(doc).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 1);
        assert!(store.exists(id).await.unwrap());
        assert!(!store.exists(Uuid::new_v4()).await.unwrap());
    }

    #[tokio::test]
    async fn test_backend_name() {
        let store = InMemoryVectorStore::new(3);
        assert_eq!(store.backend_name(), "in-memory");
    }
}
