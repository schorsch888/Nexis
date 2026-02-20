//! Qdrant vector store implementation

use async_trait::async_trait;
use qdrant_client::client::QdrantClient;
use qdrant_client::qdrant::{
    point_id::PointIdOptions, vectors::VectorsOptions, vectors_config::Config, Condition,
    CreateCollection, Distance, PointId, PointStruct, RetriedPoint, SearchPoints, UpsertPoints,
    Value, VectorParams, Vectors, VectorsConfig,
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{VectorError, VectorResult};
use crate::store::VectorStore;
use crate::types::{BatchResult, Document, DocumentMetadata, SearchQuery, SearchResult, Vector};

/// Configuration for Qdrant connection
#[derive(Debug, Clone)]
pub struct QdrantConfig {
    /// Qdrant server URL
    pub url: String,
    /// Collection name
    pub collection_name: String,
    /// Vector dimension
    pub dimension: usize,
    /// API key (optional)
    pub api_key: Option<String>,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:6334".to_string(),
            collection_name: "nexis_vectors".to_string(),
            dimension: 1536,
            api_key: None,
            timeout_secs: 30,
        }
    }
}

impl QdrantConfig {
    /// Create a new configuration
    pub fn new(url: impl Into<String>, collection_name: impl Into<String>, dimension: usize) -> Self {
        Self {
            url: url.into(),
            collection_name: collection_name.into(),
            dimension,
            ..Default::default()
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

/// Qdrant vector store implementation
pub struct QdrantVectorStore {
    client: QdrantClient,
    config: QdrantConfig,
}

impl QdrantVectorStore {
    /// Create a new Qdrant vector store
    pub async fn new(config: QdrantConfig) -> VectorResult<Self> {
        let mut builder = QdrantClient::from_url(&config.url);

        if let Some(ref api_key) = config.api_key {
            builder = builder.api_key(api_key);
        }

        builder = builder.timeout(Duration::from_secs(config.timeout_secs));

        let client = builder
            .build()
            .map_err(|e| VectorError::connection(e.to_string()))?;

        let store = Self { client, config };
        store.ensure_collection().await?;

        info!(
            collection = %store.config.collection_name,
            dimension = store.config.dimension,
            "Qdrant vector store initialized"
        );

        Ok(store)
    }

    /// Ensure the collection exists
    async fn ensure_collection(&self) -> VectorResult<()> {
        let collection_name = &self.config.collection_name;
        let dimension = self.config.dimension as u64;

        let collections = self
            .client
            .list_collections()
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == *collection_name);

        if !exists {
            info!(collection = %collection_name, dimension, "Creating Qdrant collection");

            self.client
                .create_collection(&CreateCollection {
                    collection_name: collection_name.clone(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: dimension,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await
                .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

            debug!(collection = %collection_name, "Collection created successfully");
        }

        Ok(())
    }

    /// Convert document to Qdrant point
    fn doc_to_point(&self, doc: &Document) -> VectorResult<PointStruct> {
        let id = doc.id.to_string();
        let vector = doc.vector.data.clone();

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("content".to_string(), Value::from(doc.content.clone()));
        payload.insert(
            "created_at".to_string(),
            Value::from(doc.created_at.to_rfc3339()),
        );
        payload.insert(
            "updated_at".to_string(),
            Value::from(doc.updated_at.to_rfc3339()),
        );

        if let Some(room_id) = doc.metadata.room_id {
            payload.insert("room_id".to_string(), Value::from(room_id.to_string()));
        }
        if let Some(user_id) = doc.metadata.user_id {
            payload.insert("user_id".to_string(), Value::from(user_id.to_string()));
        }
        if let Some(message_id) = doc.metadata.message_id {
            payload.insert("message_id".to_string(), Value::from(message_id.to_string()));
        }
        payload.insert(
            "tags".to_string(),
            Value::from(
                doc.metadata
                    .tags
                    .iter()
                    .map(|s| Value::from(s.clone()))
                    .collect::<Vec<_>>(),
            ),
        );

        Ok(PointStruct {
            id: Some(PointId {
                point_id_options: Some(PointIdOptions::Uuid(id)),
            }),
            vectors: Some(Vectors {
                vectors_options: Some(VectorsOptions::Vector(vector)),
            }),
            payload,
        })
    }

    /// Extract UUID string from PointId
    fn extract_uuid(point_id: &Option<PointId>) -> Option<String> {
        point_id.as_ref().and_then(|id| match &id.point_id_options {
            Some(PointIdOptions::Uuid(uuid)) => Some(uuid.clone()),
            Some(PointIdOptions::Num(num)) => Some(num.to_string()),
            None => None,
        })
    }

    /// Get string value from payload
    fn get_string_value(payload: &HashMap<String, Value>, key: &str) -> Option<String> {
        payload.get(key).and_then(|v| match &v.kind {
            Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => Some(s.clone()),
            _ => None,
        })
    }

    /// Get list value from payload
    fn get_list_value(payload: &HashMap<String, Value>, key: &str) -> Vec<String> {
        payload
            .get(key)
            .and_then(|v| match &v.kind {
                Some(qdrant_client::qdrant::value::Kind::ListValue(list)) => Some(
                    list.values
                        .iter()
                        .filter_map(|v| match &v.kind {
                            Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => {
                                Some(s.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Convert Qdrant point to document
    fn point_to_doc(&self, point: &RetriedPoint) -> VectorResult<Document> {
        let id_str = Self::extract_uuid(&point.id)
            .ok_or_else(|| VectorError::not_found("point without id"))?;

        let id = Uuid::parse_str(&id_str)
            .map_err(|e| VectorError::backend("qdrant", format!("Invalid UUID: {}", e)))?;

        let vector = point
            .vectors
            .as_ref()
            .and_then(|v| match &v.vectors_options {
                Some(VectorsOptions::Vector(vec)) => Some(vec.clone()),
                Some(VectorsOptions::Vectors(vectors)) => vectors.vectors.first().cloned(),
                None => None,
            })
            .ok_or_else(|| VectorError::backend("qdrant", "Point without vector"))?;

        let content = Self::get_string_value(&point.payload, "content").unwrap_or_default();

        let room_id = Self::get_string_value(&point.payload, "room_id")
            .and_then(|s| Uuid::parse_str(&s).ok());

        let user_id = Self::get_string_value(&point.payload, "user_id")
            .and_then(|s| Uuid::parse_str(&s).ok());

        let message_id = Self::get_string_value(&point.payload, "message_id")
            .and_then(|s| Uuid::parse_str(&s).ok());

        let tags = Self::get_list_value(&point.payload, "tags");

        let created_at = Self::get_string_value(&point.payload, "created_at")
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let updated_at = Self::get_string_value(&point.payload, "updated_at")
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let metadata = DocumentMetadata {
            room_id,
            user_id,
            message_id,
            tags,
            extra: HashMap::new(),
        };

        Ok(Document {
            id,
            vector: Vector::new(vector),
            metadata,
            content,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl VectorStore for QdrantVectorStore {
    async fn upsert(&self, document: Document) -> VectorResult<Uuid> {
        if document.vector.dimensions != self.config.dimension {
            return Err(VectorError::invalid_dimension(
                self.config.dimension,
                document.vector.dimensions,
            ));
        }

        let point = self.doc_to_point(&document)?;
        let id = document.id;

        self.client
            .upsert_points(&UpsertPoints {
                collection_name: self.config.collection_name.clone(),
                points: vec![point],
                wait: Some(true),
                ..Default::default()
            })
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        debug!(id = %id, "Document upserted to Qdrant");
        Ok(id)
    }

    async fn upsert_batch(&self, documents: Vec<Document>) -> VectorResult<BatchResult> {
        let mut result = BatchResult::new();
        let mut points = Vec::with_capacity(documents.len());

        for doc in documents {
            if doc.vector.dimensions != self.config.dimension {
                result.add_failure(
                    doc.id,
                    format!(
                        "Invalid dimension: expected {}, got {}",
                        self.config.dimension, doc.vector.dimensions
                    ),
                );
            } else {
                match self.doc_to_point(&doc) {
                    Ok(point) => {
                        result.add_success(doc.id);
                        points.push(point);
                    }
                    Err(e) => {
                        result.add_failure(doc.id, e.to_string());
                    }
                }
            }
        }

        if !points.is_empty() {
            self.client
                .upsert_points(&UpsertPoints {
                    collection_name: self.config.collection_name.clone(),
                    points,
                    wait: Some(true),
                    ..Default::default()
                })
                .await
                .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;
        }

        Ok(result)
    }

    async fn get(&self, id: Uuid) -> VectorResult<Document> {
        let points = self
            .client
            .get_points(
                &self.config.collection_name,
                &[PointId {
                    point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
                }],
                true,
                true,
            )
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        let point = points
            .into_iter()
            .next()
            .ok_or_else(|| VectorError::not_found_uuid(id))?;

        self.point_to_doc(&point)
    }

    async fn get_batch(&self, ids: Vec<Uuid>) -> VectorResult<Vec<Document>> {
        let point_ids: Vec<PointId> = ids
            .iter()
            .map(|id| PointId {
                point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
            })
            .collect();

        let points = self
            .client
            .get_points(&self.config.collection_name, &point_ids, true, true)
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        let mut docs = Vec::with_capacity(points.len());
        for point in points {
            match self.point_to_doc(&point) {
                Ok(doc) => docs.push(doc),
                Err(e) => {
                    warn!(error = %e, "Failed to convert point to document");
                }
            }
        }

        Ok(docs)
    }

    async fn delete(&self, id: Uuid) -> VectorResult<()> {
        self.client
            .delete_points(
                &self.config.collection_name,
                &[PointId {
                    point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
                }],
                Some(true),
            )
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        debug!(id = %id, "Document deleted from Qdrant");
        Ok(())
    }

    async fn delete_batch(&self, ids: Vec<Uuid>) -> VectorResult<BatchResult> {
        let point_ids: Vec<PointId> = ids
            .iter()
            .map(|id| PointId {
                point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
            })
            .collect();

        self.client
            .delete_points(&self.config.collection_name, &point_ids, Some(true))
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        let mut result = BatchResult::new();
        for id in ids {
            result.add_success(id);
        }

        Ok(result)
    }

    async fn search(&self, query: SearchQuery) -> VectorResult<Vec<SearchResult>> {
        query.validate().map_err(VectorError::invalid_query)?;

        let mut search_request = SearchPoints {
            collection_name: self.config.collection_name.clone(),
            vector: query.vector.data.clone(),
            limit: query.limit as u64,
            with_vectors: Some(true),
            with_payload: Some(true),
            score_threshold: query.min_score,
            ..Default::default()
        };

        if let Some(filter) = &query.filter {
            if let Some(qdrant_filter) = self.build_qdrant_filter(filter) {
                search_request.filter = Some(qdrant_filter);
            }
        }

        let results = self
            .client
            .search_points(&search_request)
            .await
            .map_err(|e| VectorError::search_failed(e.to_string()))?;

        let mut search_results = Vec::with_capacity(results.len());
        for scored_point in results {
            let score = scored_point.score;
            let point = RetriedPoint {
                id: scored_point.id,
                payload: scored_point.payload,
                vectors: scored_point.vectors.map(|v| Vectors {
                    vectors_options: Some(v),
                }),
                shard_key: None,
                order_value: None,
            };
            match self.point_to_doc(&point) {
                Ok(doc) => search_results.push(SearchResult::new(doc, score)),
                Err(e) => {
                    warn!(error = %e, "Failed to convert search result");
                }
            }
        }

        Ok(search_results)
    }

    async fn count(&self) -> VectorResult<usize> {
        let result = self
            .client
            .get_collection_info(&self.config.collection_name)
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        let count = result
            .result
            .and_then(|r| r.points_count)
            .unwrap_or(0);

        Ok(count as usize)
    }

    async fn exists(&self, id: Uuid) -> VectorResult<bool> {
        let points = self
            .client
            .get_points(
                &self.config.collection_name,
                &[PointId {
                    point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
                }],
                false,
                false,
            )
            .await
            .map_err(|e| VectorError::backend("qdrant", e.to_string()))?;

        Ok(!points.is_empty())
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn backend_name(&self) -> &'static str {
        "qdrant"
    }
}

impl QdrantVectorStore {
    /// Build Qdrant filter from SearchFilter
    fn build_qdrant_filter(
        &self,
        filter: &crate::types::SearchFilter,
    ) -> Option<qdrant_client::qdrant::Filter> {
        use qdrant_client::qdrant::{FieldCondition, Filter, Match};

        let mut conditions: Vec<Condition> = Vec::new();

        if let Some(room_id) = filter.room_id {
            conditions.push(Condition::matches("room_id", room_id.to_string()));
        }

        if let Some(user_id) = filter.user_id {
            conditions.push(Condition::matches("user_id", user_id.to_string()));
        }

        for tag in &filter.tags {
            conditions.push(Condition::matches("tags", tag.clone()));
        }

        if let Some(ref time_range) = filter.time_range {
            let start_ts = time_range.start.timestamp();
            let end_ts = time_range.end.timestamp();

            conditions.push(Condition::from(FieldCondition {
                key: "created_at".to_string(),
                r#match: None,
                range: Some(qdrant_client::qdrant::Range {
                    gte: Some(start_ts as f64),
                    lte: Some(end_ts as f64),
                    ..Default::default()
                }),
                geo_bounding_box: None,
                geo_radius: None,
                values_count: None,
                is_empty: None,
                is_null: None,
                nested: None,
            }));
        }

        if conditions.is_empty() {
            None
        } else {
            Some(Filter {
                should: Vec::new(),
                min_should: None,
                must: conditions,
                must_not: Vec::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qdrant_available() -> bool {
        std::env::var("NEXIS_QDRANT_URL").is_ok()
    }

    #[tokio::test]
    async fn test_qdrant_config() {
        let config = QdrantConfig::new("http://localhost:6334", "test_collection", 512);
        assert_eq!(config.url, "http://localhost:6334");
        assert_eq!(config.collection_name, "test_collection");
        assert_eq!(config.dimension, 512);
    }

    #[tokio::test]
    async fn test_qdrant_upsert_and_get() {
        if !qdrant_available() {
            eprintln!("Skipping Qdrant test: set NEXIS_QDRANT_URL to enable");
            return;
        }

        let url = std::env::var("NEXIS_QDRANT_URL").unwrap();
        let collection = format!("test_{}", Uuid::new_v4());

        let config = QdrantConfig::new(&url, &collection, 3);
        let store = QdrantVectorStore::new(config).await.unwrap();

        let doc = Document::new(
            Vector::new(vec![1.0, 0.0, 0.0]),
            "test content".to_string(),
            DocumentMetadata::new(),
        );

        let id = store.upsert(doc.clone()).await.unwrap();
        let retrieved = store.get(id).await.unwrap();

        assert_eq!(doc.content, retrieved.content);
    }
}
