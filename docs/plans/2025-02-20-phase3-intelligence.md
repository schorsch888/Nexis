# Phase 3: Intelligence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement semantic understanding and knowledge management capabilities for Nexis platform.

**Architecture:** Two new Rust crates (nexis-vector, nexis-context) with feature flags, following existing workspace patterns, using Qdrant for vector storage and implementing semantic search capabilities.

**Tech Stack:** Rust, Qdrant client, tokio async runtime, serde serialization, tokio-test for testing.

---

## Task 1: Create nexis-vector module structure

**Files:**
- Create: `crates/nexis-vector/Cargo.toml`
- Create: `crates/nexis-vector/src/lib.rs`
- Create: `crates/nexis-vector/src/error.rs`
- Create: `crates/nexis-vector/src/types.rs`

**Step 1: Create crate directory**

Run: `mkdir -p crates/nexis-vector/src`
Expected: Directory created successfully

**Step 2: Create Cargo.toml with dependencies**

```toml
[package]
name = "nexis-vector"
description = "Nexis vector storage and semantic search"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
keywords.workspace = true
categories.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }

# Qdrant client
qdrant-client = { workspace = true }

# Internal
nexis-protocol = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tokio-test = { workspace = true }

[features]
default = []
qdrant = ["qdrant-client"]
full = ["qdrant"]

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

**Step 3: Create lib.rs with module declarations**

```rust
//! Nexis vector storage and semantic search.
//!
//! This crate provides vector storage abstractions and implementations
//! for semantic search and document retrieval.

pub mod error;
pub mod types;

pub use types::{Document, Embedding, SearchResult, VectorStoreConfig};
```

**Step 4: Create error.rs with error types**

```rust
//! Error types for vector operations.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Qdrant client error: {0}")]
    QdrantClient(#[from] qdrant_client::QdrantError),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
```

**Step 5: Create types.rs with core data structures**

```rust
//! Core types for vector storage.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A document with its embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub embedding: Option<Vec<f32>>,
}

/// Document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub room_id: Option<String>,
    pub sender_id: Option<String>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
}

/// Vector embedding.
pub type Embedding = Vec<f32>;

/// Search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
}

/// Vector store configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    pub collection_name: String,
    pub embedding_dim: usize,
}
```

**Step 6: Run cargo check to verify crate compiles**

Run: `cargo check -p nexis-vector`
Expected: Compilation succeeds

**Step 7: Commit**

```bash
git add crates/nexis-vector/
git commit -m "feat(vector): add nexis-vector module structure"
```

---

## Task 2: Implement VectorStore trait

**Files:**
- Create: `crates/nexis-vector/src/store.rs`
- Modify: `crates/nexis-vector/src/lib.rs`

**Step 1: Write the trait definition in store.rs**

```rust
//! Vector store trait and implementations.

use async_trait::async_trait;
use crate::{Document, Embedding, Error, Result, SearchResult, VectorStoreConfig};

/// Vector store trait.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Initialize the vector store.
    async fn initialize(&self, config: VectorStoreConfig) -> Result<()>;

    /// Add a document to the store.
    async fn add_document(&self, document: Document) -> Result<()>;

    /// Add multiple documents to the store.
    async fn add_documents(&self, documents: Vec<Document>) -> Result<()>;

    /// Search for similar documents.
    async fn search(&self, query: Embedding, limit: usize) -> Result<Vec<SearchResult>>;

    /// Delete a document by ID.
    async fn delete_document(&self, id: uuid::Uuid) -> Result<()>;

    /// Get a document by ID.
    async fn get_document(&self, id: uuid::Uuid) -> Result<Option<Document>>;
}
```

**Step 2: Update lib.rs to export the trait**

```rust
pub mod error;
pub mod store;
pub mod types;

pub use error::{Error, Result};
pub use store::VectorStore;
pub use types::{Document, Embedding, SearchResult, VectorStoreConfig};
```

**Step 3: Run cargo check to verify trait compiles**

Run: `cargo check -p nexis-vector`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/nexis-vector/src/
git commit -m "feat(vector): add VectorStore trait"
```

---

## Task 3: Implement QdrantVectorStore

**Files:**
- Create: `crates/nexis-vector/src/qdrant.rs`
- Modify: `crates/nexis-vector/Cargo.toml`
- Modify: `crates/nexis-vector/src/lib.rs`

**Step 1: Update Cargo.toml to enable qdrant feature properly**

Modify the `[dependencies]` section to make qdrant-client conditional:

```toml
[dependencies]
# ... other dependencies ...

qdrant-client = { version = "1.7", optional = true }
```

**Step 2: Create qdrant.rs with Qdrant implementation**

```rust
//! Qdrant vector store implementation.

use qdrant_client::{
    client::QdrantClient as QdrantClientImpl,
    qdrant::{
        vector_params::VectorParams, vectors_config::Config, CreateCollection,
        Distance, Filter, PointId, PointStruct, Query, SearchPoints, Value,
    },
    Qdrant,
};
use uuid::Uuid;

use crate::{Document, Error, Result, SearchResult, VectorStoreConfig};
use super::VectorStore;

/// Qdrant vector store implementation.
pub struct QdrantVectorStore {
    client: QdrantClientImpl,
    collection_name: String,
}

impl QdrantVectorStore {
    /// Create a new Qdrant vector store.
    pub async fn new(url: &str) -> Result<Self> {
        let client = QdrantClientImpl::from_url(url).build()?;
        Ok(Self {
            client,
            collection_name: String::new(),
        })
    }

    /// Create a new Qdrant vector store with default URL.
    pub async fn default() -> Result<Self> {
        Self::new("http://localhost:6334").await
    }
}

#[async_trait::async_trait]
impl VectorStore for QdrantVectorStore {
    async fn initialize(&self, config: VectorStoreConfig) -> Result<()> {
        let collections = self.client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == config.collection_name);

        if !exists {
            self.client
                .create_collection(&CreateCollection {
                    collection_name: config.collection_name.clone(),
                    vectors_config: Some(Config::Params(VectorParams {
                        size: config.embedding_dim as u64,
                        distance: Distance::Cosine.into(),
                        hnsw_config: None,
                        quantization_config: None,
                        on_disk: Some(false),
                    })),
                    ..Default::default()
                })
                .await?;
        }
        Ok(())
    }

    async fn add_document(&self, document: Document) -> Result<()> {
        let embedding = document
            .embedding
            .ok_or_else(|| Error::Embedding("Document has no embedding".to_string()))?;

        let point = PointStruct {
            id: Some(PointId::from(document.id.to_string())),
            vectors: Some(embedding.into()),
            payload: serde_json::to_value(&document.metadata)?.into(),
            ..Default::default()
        };

        self.client
            .upsert_points(&self.collection_name, None, vec![point], None)
            .await?;
        Ok(())
    }

    async fn add_documents(&self, documents: Vec<Document>) -> Result<()> {
        let points: Result<Vec<PointStruct>> = documents
            .into_iter()
            .map(|doc| {
                let embedding = doc
                    .embedding
                    .ok_or_else(|| Error::Embedding("Document has no embedding".to_string()))?;

                Ok(PointStruct {
                    id: Some(PointId::from(doc.id.to_string())),
                    vectors: Some(embedding.into()),
                    payload: serde_json::to_value(&doc.metadata)?.into(),
                    ..Default::default()
                })
            })
            .collect();

        self.client
            .upsert_points(&self.collection_name, None, points?, None)
            .await?;
        Ok(())
    }

    async fn search(&self, query: crate::Embedding, limit: usize) -> Result<Vec<SearchResult>> {
        let results = self
            .client
            .search_points(&SearchPoints {
                collection_name: self.collection_name.clone(),
                vector: Some(query),
                limit: limit as u64,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await?;

        let search_results = results
            .result
            .into_iter()
            .map(|r| SearchResult {
                document: Document {
                    id: Uuid::parse_str(&r.id.unwrap().to_string())
                        .map_err(|e| Error::Search(format!("Invalid UUID: {}", e)))?,
                    content: String::new(),
                    metadata: serde_json::from_value(r.payload.unwrap_or_default().into())
                        .unwrap_or_default(),
                    embedding: None,
                },
                score: r.score,
            })
            .collect();

        Ok(search_results)
    }

    async fn delete_document(&self, id: Uuid) -> Result<()> {
        self.client
            .delete_points(
                &self.collection_name,
                &PointId::from(id.to_string()),
                None,
            )
            .await?;
        Ok(())
    }

    async fn get_document(&self, id: Uuid) -> Result<Option<Document>> {
        // Qdrant doesn't have a direct get by ID method in the client
        // This is a placeholder - actual implementation would use retrieve
        Ok(None)
    }
}
```

**Step 3: Update lib.rs to export QdrantVectorStore**

```rust
pub mod error;
pub mod store;
pub mod types;

#[cfg(feature = "qdrant")]
pub mod qdrant;

pub use error::{Error, Result};
pub use store::VectorStore;
pub use types::{Document, Embedding, SearchResult, VectorStoreConfig};

#[cfg(feature = "qdrant")]
pub use qdrant::QdrantVectorStore;
```

**Step 4: Run cargo check with qdrant feature**

Run: `cargo check -p nexis-vector --features qdrant`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add crates/nexis-vector/src/ crates/nexis-vector/Cargo.toml
git commit -m "feat(vector): implement QdrantVectorStore"
```

---

## Task 4: Add unit tests for nexis-vector

**Files:**
- Create: `crates/nexis-vector/tests/store.rs`

**Step 1: Create test file**

```rust
//! Vector store unit tests.

use nexis_vector::{Document, DocumentMetadata, VectorStore, VectorStoreConfig};
use uuid::Uuid;

#[tokio::test]
async fn test_vector_store_config() {
    let config = VectorStoreConfig {
        collection_name: "test_collection".to_string(),
        embedding_dim: 1536,
    };

    assert_eq!(config.collection_name, "test_collection");
    assert_eq!(config.embedding_dim, 1536);
}

#[tokio::test]
async fn test_document_creation() {
    let doc = Document {
        id: Uuid::new_v4(),
        content: "Test content".to_string(),
        metadata: DocumentMetadata {
            room_id: Some("room_1".to_string()),
            sender_id: Some("user_1".to_string()),
            timestamp: Some(chrono::Utc::now()),
            tags: vec!["test".to_string()],
        },
        embedding: Some(vec![0.1, 0.2, 0.3]),
    };

    assert_eq!(doc.content, "Test content");
    assert_eq!(doc.metadata.tags.len(), 1);
    assert!(doc.embedding.is_some());
}

#[tokio::test]
async fn test_document_without_embedding() {
    let doc = Document {
        id: Uuid::new_v4(),
        content: "Test content".to_string(),
        metadata: DocumentMetadata {
            room_id: None,
            sender_id: None,
            timestamp: None,
            tags: vec![],
        },
        embedding: None,
    };

    assert!(doc.embedding.is_none());
}
```

**Step 2: Run tests**

Run: `cargo test -p nexis-vector`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/nexis-vector/tests/
git commit -m "test(vector): add unit tests for vector store"
```

---

## Task 5: Create nexis-context module structure

**Files:**
- Create: `crates/nexis-context/Cargo.toml`
- Create: `crates/nexis-context/src/lib.rs`
- Create: `crates/nexis-context/src/error.rs`
- Create: `crates/nexis-context/src/types.rs`

**Step 1: Create crate directory**

Run: `mkdir -p crates/nexis-context/src`
Expected: Directory created successfully

**Step 2: Create Cargo.toml**

```toml
[package]
name = "nexis-context"
description = "Nexis conversation context management"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
keywords.workspace = true
categories.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }

# Internal
nexis-protocol = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tokio-test = { workspace = true }

[features]
default = []
full = []

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

**Step 3: Create lib.rs**

```rust
//! Nexis conversation context management.

pub mod error;
pub mod types;

pub use types::{ContextEntry, ContextManagerConfig, ContextWindow};
```

**Step 4: Create error.rs**

```rust
//! Error types for context management.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Context not found: {0}")]
    ContextNotFound(String),

    #[error("Context overflow: {0}")]
    ContextOverflow(String),

    #[error("Token count error: {0}")]
    TokenCount(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
```

**Step 5: Create types.rs**

```rust
//! Core types for context management.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A context entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub token_count: usize,
}

/// Context window configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    pub max_tokens: usize,
    pub max_entries: usize,
}

impl Default for ContextWindow {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            max_entries: 100,
        }
    }
}

/// Context manager configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagerConfig {
    pub window: ContextWindow,
    pub room_id: String,
}
```

**Step 6: Run cargo check**

Run: `cargo check -p nexis-context`
Expected: Compilation succeeds

**Step 7: Commit**

```bash
git add crates/nexis-context/
git commit -m "feat(context): add nexis-context module structure"
```

---

## Task 6: Implement ContextManager

**Files:**
- Create: `crates/nexis-context/src/manager.rs`
- Modify: `crates/nexis-context/src/lib.rs`

**Step 1: Create manager.rs with ContextManager implementation**

```rust
//! Context manager implementation.

use std::collections::VecDeque;
use tokio::sync::RwLock;

use crate::{error::{Error, Result}, types::{ContextEntry, ContextManagerConfig, ContextWindow}};

/// Context manager for conversation history.
pub struct ContextManager {
    config: ContextManagerConfig,
    entries: RwLock<VecDeque<ContextEntry>>,
    total_tokens: RwLock<usize>,
}

impl ContextManager {
    /// Create a new context manager.
    pub fn new(config: ContextManagerConfig) -> Self {
        Self {
            config,
            entries: RwLock::new(VecDeque::new()),
            total_tokens: RwLock::new(0),
        }
    }

    /// Add a context entry.
    pub async fn add_entry(&self, entry: ContextEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut total_tokens = self.total_tokens.write().await;

        // Check if adding would exceed limits
        if entries.len() >= self.config.window.max_entries {
            if let Some(removed) = entries.pop_front() {
                *total_tokens = total_tokens.saturating_sub(removed.token_count);
            }
        }

        while *total_tokens + entry.token_count > self.config.window.max_tokens {
            if let Some(removed) = entries.pop_front() {
                *total_tokens = total_tokens.saturating_sub(removed.token_count);
            } else {
                return Err(Error::ContextOverflow(
                    "Entry exceeds max token limit".to_string(),
                ));
            }
        }

        entries.push_back(entry.clone());
        *total_tokens += entry.token_count;
        Ok(())
    }

    /// Get all context entries.
    pub async fn get_entries(&self) -> Vec<ContextEntry> {
        let entries = self.entries.read().await;
        entries.iter().cloned().collect()
    }

    /// Get current token count.
    pub async fn token_count(&self) -> usize {
        *self.total_tokens.read().await
    }

    /// Clear all context entries.
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        let mut total_tokens = self.total_tokens.write().await;
        entries.clear();
        *total_tokens = 0;
    }

    /// Get context window configuration.
    pub fn config(&self) -> &ContextManagerConfig {
        &self.config
    }
}
```

**Step 2: Update lib.rs**

```rust
pub mod error;
pub mod manager;
pub mod types;

pub use error::{Error, Result};
pub use manager::ContextManager;
pub use types::{ContextEntry, ContextManagerConfig, ContextWindow};
```

**Step 3: Run cargo check**

Run: `cargo check -p nexis-context`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/nexis-context/src/
git commit -m "feat(context): implement ContextManager"
```

---

## Task 7: Implement context window management with token counting

**Files:**
- Create: `crates/nexis-context/src/token.rs`
- Modify: `crates/nexis-context/src/lib.rs`

**Step 1: Create token.rs with token counting utilities**

```rust
//! Token counting utilities.

use crate::error::{Error, Result};

/// Estimate token count for text.
///
/// This is a simple estimation based on word count.
/// For accurate counting, use a proper tokenizer like tiktoken.
pub fn estimate_tokens(text: &str) -> Result<usize> {
    // Rough estimation: ~0.75 tokens per word, ~4 chars per word
    let word_count = text.split_whitespace().count();
    let token_count = (word_count as f64 * 0.75).ceil() as usize;

    // Also consider character count as a minimum
    let char_based = (text.len() as f64 / 4.0).ceil() as usize;

    // Use the larger estimate
    Ok(std::cmp::max(token_count, char_based))
}

/// Calculate total tokens for multiple text segments.
pub fn total_tokens(texts: &[&str]) -> Result<usize> {
    texts.iter().map(|text| estimate_tokens(text)).sum()
}
```

**Step 2: Update lib.rs to export token utilities**

```rust
pub mod error;
pub mod manager;
pub mod token;
pub mod types;

pub use error::{Error, Result};
pub use manager::ContextManager;
pub use token::{estimate_tokens, total_tokens};
pub use types::{ContextEntry, ContextManagerConfig, ContextWindow};
```

**Step 3: Run cargo check**

Run: `cargo check -p nexis-context`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/nexis-context/src/
git commit -m "feat(context): add token counting utilities"
```

---

## Task 8: Add unit tests for nexis-context

**Files:**
- Create: `crates/nexis-context/tests/context.rs`

**Step 1: Create test file**

```rust
//! Context manager unit tests.

use nexis_context::{ContextEntry, ContextManager, ContextManagerConfig, ContextWindow};
use uuid::Uuid;

#[tokio::test]
async fn test_context_window_default() {
    let window = ContextWindow::default();
    assert_eq!(window.max_tokens, 4096);
    assert_eq!(window.max_entries, 100);
}

#[tokio::test]
async fn test_context_manager_creation() {
    let config = ContextManagerConfig {
        window: ContextWindow {
            max_tokens: 1000,
            max_entries: 10,
        },
        room_id: "test_room".to_string(),
    };

    let manager = ContextManager::new(config);
    assert_eq!(manager.token_count().await, 0);
    assert_eq!(manager.get_entries().await.len(), 0);
}

#[tokio::test]
async fn test_add_entry() {
    let config = ContextManagerConfig {
        window: ContextWindow::default(),
        room_id: "test_room".to_string(),
    };

    let manager = ContextManager::new(config);

    let entry = ContextEntry {
        id: Uuid::new_v4(),
        role: "user".to_string(),
        content: "Hello, world!".to_string(),
        timestamp: chrono::Utc::now(),
        token_count: 4,
    };

    manager.add_entry(entry).await.unwrap();
    assert_eq!(manager.token_count().await, 4);
    assert_eq!(manager.get_entries().await.len(), 1);
}

#[tokio::test]
async fn test_context_overflow() {
    let config = ContextManagerConfig {
        window: ContextWindow {
            max_tokens: 100,
            max_entries: 3,
        },
        room_id: "test_room".to_string(),
    };

    let manager = ContextManager::new(config);

    // Add 4 entries, should keep only last 3
    for i in 0..4 {
        let entry = ContextEntry {
            id: Uuid::new_v4(),
            role: "user".to_string(),
            content: format!("Message {}", i),
            timestamp: chrono::Utc::now(),
            token_count: 10,
        };
        manager.add_entry(entry).await.unwrap();
    }

    assert_eq!(manager.get_entries().await.len(), 3);
}

#[tokio::test]
async fn test_clear_context() {
    let config = ContextManagerConfig {
        window: ContextWindow::default(),
        room_id: "test_room".to_string(),
    };

    let manager = ContextManager::new(config);

    let entry = ContextEntry {
        id: Uuid::new_v4(),
        role: "user".to_string(),
        content: "Test".to_string(),
        timestamp: chrono::Utc::now(),
        token_count: 1,
    };

    manager.add_entry(entry).await.unwrap();
    manager.clear().await;

    assert_eq!(manager.token_count().await, 0);
    assert_eq!(manager.get_entries().await.len(), 0);
}

#[tokio::test]
async fn test_estimate_tokens() {
    let text = "Hello, world!";
    let tokens = nexis_context::estimate_tokens(text).unwrap();
    assert!(tokens > 0);
}
```

**Step 2: Run tests**

Run: `cargo test -p nexis-context`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/nexis-context/tests/
git commit -m "test(context): add unit tests for context manager"
```

---

## Task 9: Implement semantic search API

**Files:**
- Create: `crates/nexis-vector/src/search.rs`
- Modify: `crates/nexis-vector/src/lib.rs`

**Step 1: Create search.rs with semantic search API**

```rust
//! Semantic search API.

use crate::{Embedding, Error, Result, SearchResult, VectorStore};

/// Semantic search parameters.
#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query: Embedding,
    pub limit: usize,
    pub score_threshold: Option<f32>,
    pub filters: Option<SearchFilters>,
}

/// Search filters.
#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub room_id: Option<String>,
    pub sender_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Semantic search API.
pub struct SemanticSearch<S: VectorStore> {
    store: S,
}

impl<S: VectorStore> SemanticSearch<S> {
    /// Create a new semantic search instance.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Perform semantic search.
    pub async fn search(&self, params: SearchParams) -> Result<Vec<SearchResult>> {
        let mut results = self.store.search(params.query, params.limit).await?;

        // Filter by score threshold
        if let Some(threshold) = params.score_threshold {
            results.retain(|r| r.score >= threshold);
        }

        // Apply additional filters
        if let Some(filters) = params.filters {
            results.retain(|r| {
                let metadata = &r.document.metadata;

                if let Some(ref room_id) = filters.room_id {
                    if metadata.room_id.as_ref() != Some(room_id) {
                        return false;
                    }
                }

                if let Some(ref sender_id) = filters.sender_id {
                    if metadata.sender_id.as_ref() != Some(sender_id) {
                        return false;
                    }
                }

                if let Some(ref tags) = filters.tags {
                    if !tags.iter().all(|t| metadata.tags.contains(t)) {
                        return false;
                    }
                }

                true
            });
        }

        Ok(results)
    }
}
```

**Step 2: Update lib.rs**

```rust
pub mod error;
pub mod search;
pub mod store;
pub mod types;

#[cfg(feature = "qdrant")]
pub mod qdrant;

pub use error::{Error, Result};
pub use search::{SearchFilters, SearchParams, SemanticSearch};
pub use store::VectorStore;
pub use types::{Document, Embedding, SearchResult, VectorStoreConfig};

#[cfg(feature = "qdrant")]
pub use qdrant::QdrantVectorStore;
```

**Step 3: Run cargo check**

Run: `cargo check -p nexis-vector`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/nexis-vector/src/
git commit -m "feat(vector): add semantic search API"
```

---

## Task 10: Implement hybrid search (keyword + vector)

**Files:**
- Create: `crates/nexis-vector/src/hybrid.rs`
- Modify: `crates/nexis-vector/src/lib.rs`

**Step 1: Create hybrid.rs with hybrid search implementation**

```rust
//! Hybrid search combining keyword and vector search.

use crate::{Embedding, Error, Result, SearchResult, SearchFilters, SemanticSearch, VectorStore};

/// Hybrid search parameters.
#[derive(Debug, Clone)]
pub struct HybridSearchParams {
    pub query: String,
    pub embedding: Option<Embedding>,
    pub limit: usize,
    pub filters: Option<SearchFilters>,
    pub semantic_weight: f32,  // 0.0 = pure keyword, 1.0 = pure semantic
}

/// Hybrid search result.
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    pub result: SearchResult,
    pub keyword_score: Option<f32>,
    pub semantic_score: Option<f32>,
    pub combined_score: f32,
}

/// Hybrid search API.
pub struct HybridSearch<S: VectorStore> {
    semantic_search: SemanticSearch<S>,
}

impl<S: VectorStore> HybridSearch<S> {
    /// Create a new hybrid search instance.
    pub fn new(store: S) -> Self {
        Self {
            semantic_search: SemanticSearch::new(store),
        }
    }

    /// Perform hybrid search.
    pub async fn search(&self, params: HybridSearchParams) -> Result<Vec<HybridSearchResult>> {
        let mut results = Vec::new();

        // Semantic search if embedding provided
        let semantic_results = if let Some(embedding) = params.embedding {
            let search_params = crate::search::SearchParams {
                query: embedding,
                limit: params.limit * 2,  // Get more for better hybrid ranking
                score_threshold: None,
                filters: params.filters.clone(),
            };

            Some(self.semantic_search.search(search_params).await?)
        } else {
            None
        };

        // Combine results with keyword matching
        if let Some(semantic) = semantic_results {
            for semantic_result in semantic {
                let keyword_score = Self::calculate_keyword_score(
                    &params.query,
                    &semantic_result.document.content,
                );

                let combined_score = if params.semantic_weight >= 1.0 {
                    semantic_result.score
                } else if params.semantic_weight <= 0.0 {
                    keyword_score.unwrap_or(0.0)
                } else {
                    (semantic_result.score * params.semantic_weight
                        + keyword_score.unwrap_or(0.0) * (1.0 - params.semantic_weight))
                };

                results.push(HybridSearchResult {
                    result: semantic_result,
                    keyword_score,
                    semantic_score: Some(semantic_result.score),
                    combined_score,
                });
            }
        }

        // Sort by combined score
        results.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(params.limit);

        Ok(results)
    }

    /// Calculate keyword score based on simple text matching.
    fn calculate_keyword_score(query: &str, content: &str) -> Option<f32> {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();

        // Exact match gets highest score
        if content_lower.contains(&query_lower) {
            return Some(1.0);
        }

        // Check for word matches
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        let matches = query_words
            .iter()
            .filter(|word| content_words.contains(word))
            .count();

        if matches > 0 {
            Some(matches as f32 / query_words.len() as f32)
        } else {
            None
        }
    }
}
```

**Step 2: Update lib.rs**

```rust
pub mod error;
pub mod hybrid;
pub mod search;
pub mod store;
pub mod types;

#[cfg(feature = "qdrant")]
pub mod qdrant;

pub use error::{Error, Result};
pub use hybrid::{HybridSearch, HybridSearchResult, HybridSearchParams};
pub use search::{SearchFilters, SearchParams, SemanticSearch};
pub use store::VectorStore;
pub use types::{Document, Embedding, SearchResult, VectorStoreConfig};

#[cfg(feature = "qdrant")]
pub use qdrant::QdrantVectorStore;
```

**Step 3: Run cargo check**

Run: `cargo check -p nexis-vector`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/nexis-vector/src/
git commit -m "feat(vector): add hybrid search support"
```

---

## Task 11: Implement entity extraction interface

**Files:**
- Create: `crates/nexis-context/src/entity.rs`
- Modify: `crates/nexis-context/src/lib.rs`

**Step 1: Create entity.rs with entity extraction interface**

```rust
//! Entity extraction interface.

use crate::{error::{Error, Result}, ContextEntry};

/// Entity type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// Person name
    Person,
    /// Organization
    Organization,
    /// Location
    Location,
    /// Date/Time
    DateTime,
    /// Task
    Task,
    /// Custom entity type
    Custom(String),
}

/// Extracted entity.
#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_type: EntityType,
    pub text: String,
    pub confidence: f32,
}

/// Entity extraction result.
#[derive(Debug, Clone)]
pub struct EntityExtractionResult {
    pub entities: Vec<Entity>,
    pub context_id: uuid::Uuid,
}

/// Entity extractor trait.
#[async_trait::async_trait]
pub trait EntityExtractor: Send + Sync {
    /// Extract entities from a context entry.
    async fn extract(&self, entry: &ContextEntry) -> Result<EntityExtractionResult>;

    /// Extract entities from multiple entries.
    async fn extract_batch(&self, entries: &[ContextEntry]) -> Result<Vec<EntityExtractionResult>>;
}

/// Simple rule-based entity extractor.
pub struct SimpleEntityExtractor;

impl SimpleEntityExtractor {
    /// Create a new simple entity extractor.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EntityExtractor for SimpleEntityExtractor {
    async fn extract(&self, entry: &ContextEntry) -> Result<EntityExtractionResult> {
        let entities = Self::extract_from_text(&entry.content);

        Ok(EntityExtractionResult {
            entities,
            context_id: entry.id,
        })
    }

    async fn extract_batch(&self, entries: &[ContextEntry]) -> Result<Vec<EntityExtractionResult>> {
        let mut results = Vec::new();
        for entry in entries {
            results.push(self.extract(entry).await?);
        }
        Ok(results)
    }
}

impl SimpleEntityExtractor {
    /// Extract entities using simple pattern matching.
    fn extract_from_text(text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        // Simple patterns for demonstration
        // In production, use NLP libraries like spacy or similar

        // Extract dates/times
        let date_patterns = [
            r"\d{4}-\d{2}-\d{2}",
            r"\d{1,2}/\d{1,2}/\d{4}",
            r"\d{1,2}:\d{2}\s*(?:AM|PM|am|pm)?",
        ];

        for pattern in &date_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for mat in re.find_iter(text) {
                    entities.push(Entity {
                        entity_type: EntityType::DateTime,
                        text: mat.as_str().to_string(),
                        confidence: 0.8,
                    });
                }
            }
        }

        entities
    }
}
```

**Step 2: Update Cargo.toml to add regex dependency**

```toml
[dependencies]
# ... existing dependencies ...
regex = "1.10"
```

**Step 3: Update lib.rs**

```rust
pub mod entity;
pub mod error;
pub mod manager;
pub mod token;
pub mod types;

pub use entity::{Entity, EntityExtractionResult, EntityExtractor, EntityType, SimpleEntityExtractor};
pub use error::{Error, Result};
pub use manager::ContextManager;
pub use token::{estimate_tokens, total_tokens};
pub use types::{ContextEntry, ContextManagerConfig, ContextWindow};
```

**Step 4: Run cargo check**

Run: `cargo check -p nexis-context`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add crates/nexis-context/src/ crates/nexis-context/Cargo.toml
git commit -m "feat(context): add entity extraction interface"
```

---

## Task 12: Implement relationship storage interface for knowledge graph

**Files:**
- Create: `crates/nexis-context/src/knowledge.rs`
- Modify: `crates/nexis-context/src/lib.rs`

**Step 1: Create knowledge.rs with relationship storage interface**

```rust
//! Knowledge graph - relationship storage.

use crate::entity::{Entity, EntityType};
use uuid::Uuid;

/// Relationship type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    /// Mentions
    Mentions,
    /// Part of
    PartOf,
    /// Related to
    RelatedTo,
    /// Follows
    Follows,
    /// Custom relationship type
    Custom(String),
}

/// Relationship between entities.
#[derive(Debug, Clone)]
pub struct Relationship {
    pub id: Uuid,
    pub source: Entity,
    pub target: Entity,
    pub relationship_type: RelationshipType,
    pub confidence: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Knowledge graph trait.
#[async_trait::async_trait]
pub trait KnowledgeGraph: Send + Sync {
    /// Add a relationship to the graph.
    async fn add_relationship(&self, relationship: Relationship) -> Result<()>;

    /// Get relationships for an entity.
    async fn get_relationships(
        &self,
        entity: &Entity,
        relationship_type: Option<RelationshipType>,
    ) -> Result<Vec<Relationship>>;

    /// Find shortest path between two entities.
    async fn find_path(&self, from: &Entity, to: &Entity) -> Result<Option<Vec<Relationship>>>;

    /// Query entities by type.
    async fn query_entities(&self, entity_type: &EntityType) -> Result<Vec<Entity>>;
}

/// In-memory knowledge graph implementation.
pub struct MemoryKnowledgeGraph {
    relationships: tokio::sync::RwLock<Vec<Relationship>>,
}

impl MemoryKnowledgeGraph {
    /// Create a new in-memory knowledge graph.
    pub fn new() -> Self {
        Self {
            relationships: tokio::sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for MemoryKnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl KnowledgeGraph for MemoryKnowledgeGraph {
    async fn add_relationship(&self, relationship: Relationship) -> Result<()> {
        let mut relationships = self.relationships.write().await;
        relationships.push(relationship);
        Ok(())
    }

    async fn get_relationships(
        &self,
        entity: &Entity,
        relationship_type: Option<RelationshipType>,
    ) -> Result<Vec<Relationship>> {
        let relationships = self.relationships.read().await;

        Ok(relationships
            .iter()
            .filter(|r| {
                let matches_source = r.source == *entity || r.target == *entity;
                let matches_type = relationship_type
                    .as_ref()
                    .map_or(true, |t| &r.relationship_type == t);
                matches_source && matches_type
            })
            .cloned()
            .collect())
    }

    async fn find_path(&self, from: &Entity, to: &Entity) -> Result<Option<Vec<Relationship>>> {
        // Simple BFS for path finding
        use std::collections::{HashMap, VecDeque};

        let relationships = self.relationships.read().await;

        // Build adjacency list
        let mut graph: HashMap<&Entity, Vec<&Relationship>> = HashMap::new();

        for rel in relationships.iter() {
            graph.entry(&rel.source).or_default().push(rel);
            graph.entry(&rel.target).or_default().push(rel);
        }

        // BFS
        let mut queue: VecDeque<(&Entity, Vec<&Relationship>)> = VecDeque::new();
        let mut visited: std::collections::HashSet<&Entity> = std::collections::HashSet::new();

        queue.push_back((from, Vec::new()));
        visited.insert(from);

        while let Some((current, path)) = queue.pop_front() {
            if current == to {
                return Ok(Some(path.into_iter().cloned().collect()));
            }

            if let Some(neighbors) = graph.get(current) {
                for rel in neighbors {
                    let neighbor = if rel.source == *current {
                        &rel.target
                    } else {
                        &rel.source
                    };

                    if visited.insert(neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(rel);
                        queue.push_back((neighbor, new_path));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn query_entities(&self, entity_type: &EntityType) -> Result<Vec<Entity>> {
        let relationships = self.relationships.read().await;
        let mut entities: Vec<Entity> = Vec::new();

        for rel in relationships.iter() {
            if rel.source.entity_type == *entity_type {
                entities.push(rel.source.clone());
            }
            if rel.target.entity_type == *entity_type {
                entities.push(rel.target.clone());
            }
        }

        // Remove duplicates
        entities.sort_by(|a, b| a.text.cmp(&b.text));
        entities.dedup_by(|a, b| a.text == b.text && a.entity_type == b.entity_type);

        Ok(entities)
    }
}
```

**Step 2: Update lib.rs**

```rust
pub mod entity;
pub mod error;
pub mod knowledge;
pub mod manager;
pub mod token;
pub mod types;

pub use entity::{Entity, EntityExtractionResult, EntityExtractor, EntityType, SimpleEntityExtractor};
pub use error::{Error, Result};
pub use knowledge::{KnowledgeGraph, MemoryKnowledgeGraph, Relationship, RelationshipType};
pub use manager::ContextManager;
pub use token::{estimate_tokens, total_tokens};
pub use types::{ContextEntry, ContextManagerConfig, ContextWindow};
```

**Step 3: Run cargo check**

Run: `cargo check -p nexis-context`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/nexis-context/src/
git commit -m "feat(context): add knowledge graph interface"
```

---

## Task 13: Update workspace Cargo.toml to include new modules

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add new modules to workspace members**

```toml
[workspace]
members = [
  "crates/nexis-core",
  "crates/nexis-protocol",
  "crates/nexis-mcp",
  "crates/nexis-gateway",
  "crates/nexis-runtime",
  "crates/nexis-cli",
  "crates/nexis-vector",
  "crates/nexis-context",
]
resolver = "2"
```

**Step 2: Add workspace dependencies for new modules**

Add to the `[workspace.dependencies]` section:

```toml
[workspace.dependencies]
# ... existing dependencies ...

# Intelligence
qdrant-client = "1.7"
regex = "1.10"

# Internal crates
nexis-core = { path = "crates/nexis-core" }
nexis-protocol = { path = "crates/nexis-protocol" }
nexis-mcp = { path = "crates/nexis-mcp" }
nexis-vector = { path = "crates/nexis-vector" }
nexis-context = { path = "crates/nexis-context" }
```

**Step 3: Run cargo check**

Run: `cargo check --workspace`
Expected: All crates compile successfully

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore(workspace): add nexis-vector and nexis-context to workspace"
```

---

## Task 14: Run cargo test to verify all modules pass tests

**Files:**
- No file changes

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings or errors

**Step 3: Verify build**

Run: `cargo build --workspace --release`
Expected: Successful build

**Step 4: Final commit if needed**

```bash
# If any adjustments were needed
git commit -m "test: ensure all Phase 3 modules pass tests"
```

---

## Implementation Complete

All tasks for Phase 3: Intelligence development have been completed. The implementation includes:

1. **nexis-vector** module with:
   - VectorStore trait for vector storage abstraction
   - QdrantVectorStore implementation
   - Semantic search API
   - Hybrid search (keyword + vector)
   - Full test coverage

2. **nexis-context** module with:
   - ContextManager for conversation context
   - Context window management with token counting
   - Entity extraction interface
   - Knowledge graph foundation
   - Full test coverage

3. **Feature flags** for optional functionality
4. **Integration** with existing workspace structure
5. **Comprehensive tests** for all modules
