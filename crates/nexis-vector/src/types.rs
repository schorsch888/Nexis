//! Core types for vector storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Vector representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vector {
    /// Vector dimensions
    pub dimensions: usize,
    /// Vector data
    pub data: Vec<f32>,
}

impl Vector {
    /// Create a new vector
    pub fn new(data: Vec<f32>) -> Self {
        let dimensions = data.len();
        Self { dimensions, data }
    }

    /// Calculate cosine similarity with another vector
    pub fn cosine_similarity(&self, other: &Vector) -> f32 {
        if self.dimensions != other.dimensions {
            return 0.0;
        }

        let dot: f32 = self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum();
        let mag_a: f32 = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = other.data.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            0.0
        } else {
            dot / (mag_a * mag_b)
        }
    }

    /// Validate vector dimensions
    pub fn validate(&self) -> Result<(), String> {
        if self.data.is_empty() {
            return Err("Vector cannot be empty".to_string());
        }
        if self.dimensions != self.data.len() {
            return Err(format!(
                "Dimension mismatch: expected {}, got {}",
                self.dimensions,
                self.data.len()
            ));
        }
        Ok(())
    }
}

/// Document metadata with typed fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentMetadata {
    /// Room ID this document belongs to
    pub room_id: Option<Uuid>,
    /// User ID who created the document
    pub user_id: Option<Uuid>,
    /// Message ID if derived from a message
    pub message_id: Option<Uuid>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl DocumentMetadata {
    /// Create empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Create metadata with room ID
    pub fn with_room(mut self, room_id: Uuid) -> Self {
        self.room_id = Some(room_id);
        self
    }

    /// Create metadata with user ID
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Create metadata with message ID
    pub fn with_message(mut self, message_id: Uuid) -> Self {
        self.message_id = Some(message_id);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add custom field
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({}))
    }

    /// Parse from JSON value
    pub fn from_json(value: &serde_json::Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }
}

/// Document with vector embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Document ID
    pub id: Uuid,
    /// Vector embedding
    pub vector: Vector,
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Document content
    pub content: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Document {
    /// Create a new document
    pub fn new(vector: Vector, content: String, metadata: DocumentMetadata) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            vector,
            metadata,
            content,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create document with explicit ID
    pub fn with_id(id: Uuid, vector: Vector, content: String, metadata: DocumentMetadata) -> Self {
        let now = Utc::now();
        Self {
            id,
            vector,
            metadata,
            content,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the document content
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now();
    }

    /// Update the document vector
    pub fn update_vector(&mut self, vector: Vector) {
        self.vector = vector;
        self.updated_at = Utc::now();
    }
}

/// Filter for search queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilter {
    /// Filter by room ID
    pub room_id: Option<Uuid>,
    /// Filter by user ID
    pub user_id: Option<Uuid>,
    /// Filter by tags (matches any)
    pub tags: Vec<String>,
    /// Time range filter
    pub time_range: Option<TimeRange>,
    /// Custom filter conditions
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Time range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start of time range
    pub start: DateTime<Utc>,
    /// End of time range (inclusive)
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a new time range
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Check if a timestamp is within the range
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && timestamp <= self.end
    }
}

impl SearchFilter {
    /// Create an empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by room ID
    pub fn with_room(mut self, room_id: Uuid) -> Self {
        self.room_id = Some(room_id);
        self
    }

    /// Filter by user ID
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Filter by tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Filter by time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.time_range = Some(TimeRange::new(start, end));
        self
    }

    /// Check if a document matches this filter
    pub fn matches(&self, doc: &Document) -> bool {
        if let Some(room_id) = self.room_id {
            if doc.metadata.room_id != Some(room_id) {
                return false;
            }
        }

        if let Some(user_id) = self.user_id {
            if doc.metadata.user_id != Some(user_id) {
                return false;
            }
        }

        if !self.tags.is_empty() {
            let has_match = self.tags.iter().any(|tag| doc.metadata.tags.contains(tag));
            if !has_match {
                return false;
            }
        }

        if let Some(ref range) = self.time_range {
            if !range.contains(doc.created_at) {
                return false;
            }
        }

        true
    }

    /// Convert to JSON value for backend-specific filtering
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({}))
    }
}

/// Search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Query vector
    pub vector: Vector,
    /// Number of results to return
    pub limit: usize,
    /// Offset for pagination
    pub offset: usize,
    /// Minimum similarity threshold (0.0 to 1.0)
    pub min_score: Option<f32>,
    /// Filter conditions
    pub filter: Option<SearchFilter>,
    /// Include content in results (can be large)
    pub include_content: bool,
    /// Include metadata in results
    pub include_metadata: bool,
}

impl SearchQuery {
    /// Create a new search query
    pub fn new(vector: Vector) -> Self {
        Self {
            vector,
            limit: 10,
            offset: 0,
            min_score: None,
            filter: None,
            include_content: true,
            include_metadata: true,
        }
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    /// Set pagination offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Set minimum score threshold
    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = Some(score.clamp(0.0, 1.0));
        self
    }

    /// Set filter
    pub fn with_filter(mut self, filter: SearchFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Set room filter
    pub fn with_room(mut self, room_id: Uuid) -> Self {
        self.filter = Some(self.filter.take().unwrap_or_default().with_room(room_id));
        self
    }

    /// Exclude content from results
    pub fn without_content(mut self) -> Self {
        self.include_content = false;
        self
    }

    /// Exclude metadata from results
    pub fn without_metadata(mut self) -> Self {
        self.include_metadata = false;
        self
    }

    /// Validate the query
    pub fn validate(&self) -> Result<(), String> {
        self.vector.validate()?;
        if self.limit == 0 {
            return Err("Limit must be at least 1".to_string());
        }
        if let Some(score) = self.min_score {
            if !(0.0..=1.0).contains(&score) {
                return Err("min_score must be between 0.0 and 1.0".to_string());
            }
        }
        Ok(())
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Matched document
    pub document: Document,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
    /// Match explanation (optional)
    pub explanation: Option<String>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(document: Document, score: f32) -> Self {
        Self {
            document,
            score,
            explanation: None,
        }
    }

    /// Create with explanation
    pub fn with_explanation(document: Document, score: f32, explanation: String) -> Self {
        Self {
            document,
            score,
            explanation: Some(explanation),
        }
    }
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Successfully processed IDs
    pub succeeded: Vec<Uuid>,
    /// Failed operations with errors
    pub failed: Vec<(Uuid, String)>,
}

impl BatchResult {
    /// Create a new batch result
    pub fn new() -> Self {
        Self {
            succeeded: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Add a success
    pub fn add_success(&mut self, id: Uuid) {
        self.succeeded.push(id);
    }

    /// Add a failure
    pub fn add_failure(&mut self, id: Uuid, error: String) {
        self.failed.push((id, error));
    }

    /// Check if all operations succeeded
    pub fn is_all_success(&self) -> bool {
        self.failed.is_empty()
    }

    /// Total count of operations
    pub fn total(&self) -> usize {
        self.succeeded.len() + self.failed.len()
    }
}

impl Default for BatchResult {
    fn default() -> Self {
        Self::new()
    }
}
