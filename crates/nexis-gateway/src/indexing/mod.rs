//! Indexing service for message vectorization and storage
//!
//! This module provides:
//! - Message indexing pipeline
//! - Async embedding generation
//! - Vector storage integration

mod service;

pub use service::{IndexingService, MessageIndexer};
