//! Indexing service for message vectorization and storage
//!
//! This module provides:
//! - Message indexing pipeline
//! - Async embedding generation
//! - Vector storage integration
//! - Background task queue

mod queue;
mod retry;
mod service;

pub use queue::{IndexTask, IndexingQueue, QueueStats, SyncIndexingQueue, TaskStatus};
pub use retry::{RetryConfig, RetryPolicy};
pub use service::{IndexingError, IndexingService, MessageIndexer};
