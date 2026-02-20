//! Error types for vector storage

use thiserror::Error;
use uuid::Uuid;

/// Vector storage error type
#[derive(Error, Debug)]
pub enum VectorError {
    /// Document not found
    #[error("Document not found: {id}")]
    NotFound { id: String },

    /// Invalid vector dimension
    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    /// Invalid query parameters
    #[error("Invalid query: {message}")]
    InvalidQuery { message: String },

    /// Search operation failed
    #[error("Search failed: {message}")]
    SearchFailed { message: String },

    /// Storage backend error
    #[error("Storage error: {message}")]
    StorageError { message: String },

    /// Connection error
    #[error("Connection error: {message}")]
    ConnectionError { message: String },

    /// Timeout error
    #[error("Operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {message}")]
    ConfigurationError { message: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after_ms}ms")]
    RateLimitExceeded { retry_after_ms: u64 },

    /// Backend-specific error
    #[error("Backend error ({backend}): {message}")]
    BackendError { backend: String, message: String },

    /// Qdrant error (feature-gated)
    #[cfg(feature = "qdrant")]
    #[error("Qdrant error: {0}")]
    QdrantError(String),
}

impl VectorError {
    /// Create a not found error
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound { id: id.into() }
    }

    /// Create a not found error from UUID
    pub fn not_found_uuid(id: Uuid) -> Self {
        Self::NotFound { id: id.to_string() }
    }

    /// Create an invalid dimension error
    pub fn invalid_dimension(expected: usize, actual: usize) -> Self {
        Self::InvalidDimension { expected, actual }
    }

    /// Create an invalid query error
    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self::InvalidQuery {
            message: message.into(),
        }
    }

    /// Create a search failed error
    pub fn search_failed(message: impl Into<String>) -> Self {
        Self::SearchFailed {
            message: message.into(),
        }
    }

    /// Create a storage error
    pub fn storage(message: impl Into<String>) -> Self {
        Self::StorageError {
            message: message.into(),
        }
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::ConnectionError {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create a backend error
    pub fn backend(backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self::BackendError {
            backend: backend.into(),
            message: message.into(),
        }
    }

    /// Check if this error is retriable
    pub fn is_retriable(&self) -> bool {
        match self {
            Self::ConnectionError { .. } => true,
            Self::Timeout { .. } => true,
            Self::RateLimitExceeded { .. } => true,
            Self::BackendError { .. } => true,
            #[cfg(feature = "qdrant")]
            Self::QdrantError(_) => true,
            _ => false,
        }
    }

    /// Check if this error indicates a not found condition
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }
}

/// Result type for vector operations
pub type VectorResult<T> = Result<T, VectorError>;
