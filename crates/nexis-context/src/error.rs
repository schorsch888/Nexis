//! Error types for context management

use thiserror::Error;

/// Context management error type
#[derive(Error, Debug)]
pub enum ContextError {
    #[error("Context not found: {0}")]
    NotFound(String),

    #[error("Context window full")]
    WindowFull,

    #[error("Token counting error: {0}")]
    TokenCountError(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for context operations
pub type ContextResult<T> = Result<T, ContextError>;
