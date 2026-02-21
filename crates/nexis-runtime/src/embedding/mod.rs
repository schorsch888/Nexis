//! Embedding provider trait and implementations
//!
//! This module provides embedding generation capabilities for vector search
//! and semantic similarity operations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ProviderError;

pub mod openai;

pub use openai::OpenAIEmbeddingProvider;

pub const DEFAULT_EMBEDDING_DIMENSION: usize = 1536;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model: Option<String>,
}

impl EmbeddingRequest {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            model: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: usize,
    pub usage: Option<EmbeddingUsage>,
}

impl EmbeddingResponse {
    pub fn new(embedding: Vec<f32>, model: impl Into<String>) -> Self {
        let dimension = embedding.len();
        Self {
            embedding,
            model: model.into(),
            dimension,
            usage: None,
        }
    }

    pub fn with_usage(mut self, usage: EmbeddingUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

impl EmbeddingUsage {
    pub fn new(prompt_tokens: u32, total_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            total_tokens,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbeddingRequest {
    pub texts: Vec<String>,
    pub model: Option<String>,
}

impl BatchEmbeddingRequest {
    pub fn new(texts: Vec<String>) -> Self {
        Self {
            texts,
            model: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub dimension: usize,
    pub usage: Option<EmbeddingUsage>,
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &'static str;

    fn dimension(&self) -> usize;

    async fn embed(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, ProviderError>;

    async fn embed_batch(&self, req: BatchEmbeddingRequest) -> Result<BatchEmbeddingResponse, ProviderError>;
}

#[derive(Debug, Default)]
pub struct MockEmbeddingProvider {
    dimension: usize,
    embedding_queue: std::sync::Mutex<Vec<Result<EmbeddingResponse, ProviderError>>>,
}

impl MockEmbeddingProvider {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            embedding_queue: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn enqueue(&self, result: Result<EmbeddingResponse, ProviderError>) {
        self.embedding_queue.lock().unwrap().push(result);
    }

    fn generate_constant_embedding(&self) -> Vec<f32> {
        vec![0.1; self.dimension]
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    fn name(&self) -> &'static str {
        "mock-embedding"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn embed(&self, _req: EmbeddingRequest) -> Result<EmbeddingResponse, ProviderError> {
        let mut queue = self.embedding_queue.lock().unwrap();
        if let Some(result) = queue.pop() {
            result
        } else {
            Ok(EmbeddingResponse::new(
                self.generate_constant_embedding(),
                "mock-embedding-model",
            ))
        }
    }

    async fn embed_batch(&self, req: BatchEmbeddingRequest) -> Result<BatchEmbeddingResponse, ProviderError> {
        let count = req.texts.len();
        let embeddings = (0..count).map(|_| self.generate_constant_embedding()).collect();
        Ok(BatchEmbeddingResponse {
            embeddings,
            model: "mock-embedding-model".to_string(),
            dimension: self.dimension,
            usage: Some(EmbeddingUsage::new(count as u32 * 10, count as u32 * 10)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_request_builder() {
        let req = EmbeddingRequest::new("hello world").with_model("text-embedding-3-small");
        assert_eq!(req.text, "hello world");
        assert_eq!(req.model, Some("text-embedding-3-small".to_string()));
    }

    #[test]
    fn embedding_response_creation() {
        let embedding = vec![0.1, 0.2, 0.3];
        let resp = EmbeddingResponse::new(embedding.clone(), "test-model");
        assert_eq!(resp.embedding, embedding);
        assert_eq!(resp.dimension, 3);
        assert_eq!(resp.model, "test-model");
    }

    #[tokio::test]
    async fn mock_provider_returns_constant_embedding() {
        let provider = MockEmbeddingProvider::new(128);
        let req = EmbeddingRequest::new("test");
        let resp = provider.embed(req).await.unwrap();
        assert_eq!(resp.dimension, 128);
        assert_eq!(resp.embedding.len(), 128);
    }

    #[tokio::test]
    async fn mock_provider_returns_queued_response() {
        let provider = MockEmbeddingProvider::new(128);
        provider.enqueue(Ok(EmbeddingResponse::new(vec![1.0; 64], "custom-model")));

        let req = EmbeddingRequest::new("test");
        let resp = provider.embed(req).await.unwrap();
        assert_eq!(resp.dimension, 64);
        assert_eq!(resp.model, "custom-model");
    }

    #[tokio::test]
    async fn mock_provider_batch_embedding() {
        let provider = MockEmbeddingProvider::new(128);
        let req = BatchEmbeddingRequest::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let resp = provider.embed_batch(req).await.unwrap();
        assert_eq!(resp.embeddings.len(), 3);
        assert!(resp.usage.is_some());
    }
}
