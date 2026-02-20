//! Core types for vector storage

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Vector representation
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Document with vector embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Document ID
    pub id: Uuid,
    /// Vector embedding
    pub vector: Vector,
    /// Document metadata
    pub metadata: serde_json::Value,
    /// Document content
    pub content: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Document {
    /// Create a new document
    pub fn new(vector: Vector, content: String, metadata: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector,
            metadata,
            content,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Query vector
    pub vector: Vector,
    /// Number of results to return
    pub limit: usize,
    /// Minimum similarity threshold
    pub min_score: Option<f32>,
    /// Filter by metadata
    pub filter: Option<serde_json::Value>,
}

impl SearchQuery {
    /// Create a new search query
    pub fn new(vector: Vector) -> Self {
        Self {
            vector,
            limit: 10,
            min_score: None,
            filter: None,
        }
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum score threshold
    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = Some(score);
        self
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Matched document
    pub document: Document,
    /// Similarity score
    pub score: f32,
}
