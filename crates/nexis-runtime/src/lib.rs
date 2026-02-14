//! Runtime abstractions for AI providers.

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub content: String,
    pub model: Option<String>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamChunk {
    Delta { text: String },
    Done,
}

pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error("mock provider has no queued response")]
    MockQueueEmpty,
    #[error("provider error: {0}")]
    Message(String),
}

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError>;

    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError>;
}

#[derive(Debug, Default)]
pub struct MockProvider {
    generate_queue: Mutex<VecDeque<Result<GenerateResponse, ProviderError>>>,
    stream_queue: Mutex<VecDeque<Result<Vec<StreamChunk>, ProviderError>>>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue_generate(&self, result: Result<GenerateResponse, ProviderError>) {
        self.generate_queue
            .lock()
            .expect("mock generate queue poisoned")
            .push_back(result);
    }

    pub fn enqueue_stream(&self, result: Result<Vec<StreamChunk>, ProviderError>) {
        self.stream_queue
            .lock()
            .expect("mock stream queue poisoned")
            .push_back(result);
    }
}

#[async_trait]
impl AIProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn generate(&self, _req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        self.generate_queue
            .lock()
            .expect("mock generate queue poisoned")
            .pop_front()
            .unwrap_or(Err(ProviderError::MockQueueEmpty))
    }

    async fn generate_stream(
        &self,
        _req: GenerateRequest,
    ) -> Result<ProviderStream, ProviderError> {
        let next = self
            .stream_queue
            .lock()
            .expect("mock stream queue poisoned")
            .pop_front()
            .unwrap_or(Err(ProviderError::MockQueueEmpty))?;

        Ok(Box::pin(stream::iter(next.into_iter().map(Ok))))
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::{
        AIProvider, GenerateRequest, GenerateResponse, MockProvider, ProviderError, StreamChunk,
    };

    fn request() -> GenerateRequest {
        GenerateRequest {
            prompt: "hello".to_string(),
            model: Some("mock-1".to_string()),
            max_tokens: Some(64),
            temperature: Some(0.0),
            metadata: None,
        }
    }

    #[tokio::test]
    async fn mock_generate_returns_queued_response() {
        let provider = MockProvider::new();
        provider.enqueue_generate(Ok(GenerateResponse {
            content: "hello from mock".to_string(),
            model: Some("mock-1".to_string()),
            finish_reason: Some("stop".to_string()),
        }));

        let response = provider.generate(request()).await.unwrap();

        assert_eq!(response.content, "hello from mock");
        assert_eq!(response.model.as_deref(), Some("mock-1"));
        assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    }

    #[tokio::test]
    async fn mock_generate_stream_emits_chunks_in_order() {
        let provider = MockProvider::new();
        provider.enqueue_stream(Ok(vec![
            StreamChunk::Delta {
                text: "hello".to_string(),
            },
            StreamChunk::Delta {
                text: " ".to_string(),
            },
            StreamChunk::Done,
        ]));

        let mut stream = provider.generate_stream(request()).await.unwrap();
        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        let third = stream.next().await.unwrap().unwrap();
        let end = stream.next().await;

        assert_eq!(
            first,
            StreamChunk::Delta {
                text: "hello".to_string()
            }
        );
        assert_eq!(
            second,
            StreamChunk::Delta {
                text: " ".to_string()
            }
        );
        assert_eq!(third, StreamChunk::Done);
        assert!(end.is_none());
    }

    #[tokio::test]
    async fn mock_reports_empty_queue_error() {
        let provider = MockProvider::new();

        let err = provider.generate(request()).await.unwrap_err();

        assert_eq!(err, ProviderError::MockQueueEmpty);
    }

    #[tokio::test]
    async fn mock_stream_reports_queued_error() {
        let provider = MockProvider::new();
        provider.enqueue_stream(Err(ProviderError::Message("upstream timeout".to_string())));

        let err = provider.generate_stream(request()).await.unwrap_err();

        assert_eq!(err, ProviderError::Message("upstream timeout".to_string()));
    }
}
