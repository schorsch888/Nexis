//! Error types for vector storage

use thiserror::Error;

/// Vector storage error type
#[derive(Error, Debug)]
pub enum VectorError {
    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("Search error: {0}")]
    SearchFailed(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Qdrant error: {0}")]
    #[cfg(feature = "qdrant")]
    QdrantError(String),
}

/// Result type for vector operations
pub type VectorResult<T> = Result<T, VectorError>;
