//! Runtime abstractions and HTTP providers for AI integrations.
//!
//! This crate provides:
//! - AI provider traits and implementations
//! - Tool calling system for AI agents
//! - Control plane client for task management

pub mod providers;
pub mod tool;

// Re-export provider types
pub use providers::OpenAIProvider;

// Re-export tool types for convenience
pub use tool::{
    CodeExecuteTool, FileReadTool, Tool, ToolCall, ToolDefinition, ToolError, ToolRegistry,
    ToolResult, WebSearchTool,
};

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::{self, Stream};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub output: serde_json::Value,
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaskKind {
    Generate(GenerateRequest),
    ToolCall(ToolCallRequest),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct QueuedTask {
    id: String,
    attempts: u32,
    kind: TaskKind,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error("mock provider has no queued response")]
    MockQueueEmpty,
    #[error("provider error: {0}")]
    Message(String),
    #[error("http transport error: {0}")]
    Transport(String),
    #[error("http status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("response decode error: {0}")]
    Decode(String),
    #[error("retry exhausted after {attempts} attempts: {last_error}")]
    RetryExhausted { attempts: u32, last_error: String },
}

#[async_trait]
pub trait AIProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &'static str;

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError>;

    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError>;
}

#[async_trait]
pub trait ToolProvider: Send + Sync {
    async fn call_tool(&self, req: ToolCallRequest) -> Result<ToolCallResponse, ProviderError>;
}

#[derive(Debug, Clone)]
pub struct HttpJsonProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    max_retries: u32,
    retry_base_delay: Duration,
}

impl HttpJsonProvider {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client should build"),
            base_url: base_url.into(),
            api_key: api_key.into(),
            max_retries: 3,
            retry_base_delay: Duration::from_millis(200),
        }
    }

    pub fn with_retry_policy(mut self, max_retries: u32, retry_base_delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.retry_base_delay = retry_base_delay;
        self
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    async fn post_json_with_retry<TReq, TRes>(
        &self,
        path: &str,
        payload: &TReq,
    ) -> Result<TRes, ProviderError>
    where
        TReq: Serialize + Sync,
        TRes: DeserializeOwned,
    {
        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            match self.try_post_json(path, payload).await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    let retriable = is_retriable(&err);
                    last_error = Some(err.to_string());
                    if retriable && attempt < self.max_retries {
                        sleep(backoff(self.retry_base_delay, attempt)).await;
                        continue;
                    }
                    if retriable {
                        return Err(ProviderError::RetryExhausted {
                            attempts: attempt + 1,
                            last_error: last_error
                                .unwrap_or_else(|| "unknown retry error".to_string()),
                        });
                    }
                    return Err(err);
                }
            }
        }

        Err(ProviderError::RetryExhausted {
            attempts: self.max_retries + 1,
            last_error: last_error.unwrap_or_else(|| "unknown retry error".to_string()),
        })
    }

    async fn try_post_json<TReq, TRes>(&self, path: &str, payload: &TReq) -> Result<TRes, ProviderError>
    where
        TReq: Serialize + Sync,
        TRes: DeserializeOwned,
    {
        let response = self
            .client
            .post(self.endpoint(path))
            .bearer_auth(&self.api_key)
            .json(payload)
            .send()
            .await
            .map_err(|err| ProviderError::Transport(err.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unable to read body>".to_string());
            return Err(ProviderError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }

        response
            .json::<TRes>()
            .await
            .map_err(|err| ProviderError::Decode(err.to_string()))
    }
}

#[async_trait]
impl AIProvider for HttpJsonProvider {
    fn name(&self) -> &'static str {
        "http-json"
    }

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        self.post_json_with_retry("/v1/generate", &req).await
    }

    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
        match self
            .post_json_with_retry::<_, Vec<StreamChunk>>("/v1/generate_stream", &req)
            .await
        {
            Ok(chunks) => Ok(Box::pin(stream::iter(chunks.into_iter().map(Ok)))),
            Err(ProviderError::HttpStatus { status: 404, .. }) => {
                let generated = self.generate(req).await?;
                Ok(Box::pin(stream::iter(vec![
                    Ok(StreamChunk::Delta {
                        text: generated.content,
                    }),
                    Ok(StreamChunk::Done),
                ])))
            }
            Err(err) => Err(err),
        }
    }
}

#[async_trait]
impl ToolProvider for HttpJsonProvider {
    async fn call_tool(&self, req: ToolCallRequest) -> Result<ToolCallResponse, ProviderError> {
        self.post_json_with_retry("/v1/tools/call", &req).await
    }
}

#[derive(Debug)]
pub struct ControlPlaneClient {
    http: HttpJsonProvider,
    queue: Mutex<VecDeque<QueuedTask>>,
    max_task_attempts: u32,
    retry_delay: Duration,
}

impl ControlPlaneClient {
    pub fn new(http: HttpJsonProvider) -> Self {
        Self {
            http,
            queue: Mutex::new(VecDeque::new()),
            max_task_attempts: 3,
            retry_delay: Duration::from_millis(100),
        }
    }

    pub fn with_retry_policy(mut self, max_task_attempts: u32, retry_delay: Duration) -> Self {
        self.max_task_attempts = max_task_attempts.max(1);
        self.retry_delay = retry_delay;
        self
    }

    pub fn enqueue_generate(&self, task_id: impl Into<String>, req: GenerateRequest) {
        self.push_task(QueuedTask {
            id: task_id.into(),
            attempts: 0,
            kind: TaskKind::Generate(req),
        });
    }

    pub fn enqueue_tool_call(&self, task_id: impl Into<String>, req: ToolCallRequest) {
        self.push_task(QueuedTask {
            id: task_id.into(),
            attempts: 0,
            kind: TaskKind::ToolCall(req),
        });
    }

    pub fn queued_tasks(&self) -> usize {
        self.queue.lock().expect("task queue poisoned").len()
    }

    pub async fn drain_once(&self) -> Result<Option<serde_json::Value>, ProviderError> {
        let task = self.queue.lock().expect("task queue poisoned").pop_front();

        match task {
            None => Ok(None),
            Some(mut task) => match self.dispatch_task(task.clone()).await {
                Ok(result) => Ok(Some(result)),
                Err(err) if is_retriable(&err) => {
                    task.attempts += 1;
                    if task.attempts < self.max_task_attempts {
                        self.push_task(task);
                        sleep(self.retry_delay).await;
                        Ok(None)
                    } else {
                        Err(ProviderError::RetryExhausted {
                            attempts: task.attempts,
                            last_error: err.to_string(),
                        })
                    }
                }
                Err(err) => Err(err),
            },
        }
    }

    fn push_task(&self, task: QueuedTask) {
        self.queue
            .lock()
            .expect("task queue poisoned")
            .push_back(task);
    }

    async fn dispatch_task(&self, task: QueuedTask) -> Result<serde_json::Value, ProviderError> {
        #[derive(Debug, Deserialize)]
        struct ControlPlaneEnvelope {
            result: serde_json::Value,
        }

        let path = match task.kind {
            TaskKind::Generate(_) => "/v1/tasks/generate",
            TaskKind::ToolCall(_) => "/v1/tasks/tool-call",
        };

        let envelope: ControlPlaneEnvelope = self.http.post_json_with_retry(path, &task).await?;
        Ok(envelope.result)
    }
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

fn backoff(base: Duration, attempt: u32) -> Duration {
    base.saturating_mul(1_u32 << attempt)
}

fn is_retriable(err: &ProviderError) -> bool {
    match err {
        ProviderError::Transport(_) | ProviderError::RetryExhausted { .. } => true,
        ProviderError::HttpStatus { status, .. } => *status >= 500,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AIProvider, ControlPlaneClient, GenerateRequest, GenerateResponse, HttpJsonProvider,
        MockProvider, ProviderError, StreamChunk, ToolCallRequest,
    };
    use futures::StreamExt;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;
    use std::time::Duration;

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
    async fn http_provider_calls_real_generate_endpoint() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/generate");
                then.status(200).json_body(json!({
                    "content": "hello from control plane",
                    "model": "gpt-4o-mini",
                    "finish_reason": "stop"
                }));
            })
            .await;

        let provider = HttpJsonProvider::new(server.base_url(), "test-key");
        let response = provider.generate(request()).await.unwrap();

        mock.assert_async().await;
        assert_eq!(response.content, "hello from control plane");
    }

    #[tokio::test]
    async fn http_provider_retries_on_server_error() {
        use httpmock::prelude::HttpMockRequest;
        use std::sync::atomic::{AtomicU32, Ordering};
        
        // Use a static counter since httpmock's matches() only accepts fn pointers
        static CALL_COUNT: AtomicU32 = AtomicU32::new(0);
        CALL_COUNT.store(0, Ordering::SeqCst); // Reset for each test
        
        fn first_call_only(_req: &HttpMockRequest) -> bool {
            let count = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            count == 0 // Only match the first request
        }
        
        let server = MockServer::start_async().await;
        
        // First mock matches only the first request (returns 500)
        server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/generate")
                    .matches(first_call_only);
                then.status(500).body("upstream timeout");
            })
            .await;

        // Second mock matches subsequent requests (returns 200)
        let success = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/generate");
                then.status(200).json_body(json!({
                    "content": "retry success",
                    "model": "gpt-4o-mini",
                    "finish_reason": "stop"
                }));
            })
            .await;

        let provider = HttpJsonProvider::new(server.base_url(), "test-key")
            .with_retry_policy(1, Duration::from_millis(10));

        let response = provider.generate(request()).await.unwrap();
        assert_eq!(response.content, "retry success");
        success.assert_async().await;
    }

    #[tokio::test]
    async fn task_queue_dispatches_tool_call_to_control_plane() {
        let server = MockServer::start_async().await;
        let dispatched = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/tasks/tool-call");
                then.status(200)
                    .json_body(json!({"result": {"status": "ok", "tool": "search"}}));
            })
            .await;

        let queue = ControlPlaneClient::new(HttpJsonProvider::new(server.base_url(), "test-key"));
        queue.enqueue_tool_call(
            "task_1",
            ToolCallRequest {
                tool_name: "search".to_string(),
                input: json!({"query": "nexis"}),
                metadata: None,
            },
        );

        let result = queue.drain_once().await.unwrap().unwrap();

        dispatched.assert_async().await;
        assert_eq!(result["status"], "ok");
        assert_eq!(queue.queued_tasks(), 0);
    }

    #[tokio::test]
    async fn task_queue_requeues_retriable_failures() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/tasks/generate");
                then.status(500).body("control plane unavailable");
            })
            .await;

        let queue = ControlPlaneClient::new(
            HttpJsonProvider::new(server.base_url(), "test-key")
                .with_retry_policy(0, Duration::from_millis(1)),
        )
        .with_retry_policy(2, Duration::from_millis(1));
        queue.enqueue_generate("task_1", request());

        let first = queue.drain_once().await.unwrap();
        assert!(first.is_none());
        assert_eq!(queue.queued_tasks(), 1);
    }

    #[tokio::test]
    async fn task_queue_returns_retry_exhausted_after_limit() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/tasks/generate");
                then.status(500).body("control plane unavailable");
            })
            .await;

        let queue = ControlPlaneClient::new(
            HttpJsonProvider::new(server.base_url(), "test-key")
                .with_retry_policy(0, Duration::from_millis(1)),
        )
        .with_retry_policy(2, Duration::from_millis(1));
        queue.enqueue_generate("task_1", request());

        queue.drain_once().await.unwrap();
        let err = queue.drain_once().await.unwrap_err();

        match err {
            ProviderError::RetryExhausted { attempts, .. } => assert_eq!(attempts, 2),
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(queue.queued_tasks(), 0);
    }

    #[tokio::test]
    async fn task_queue_does_not_requeue_client_errors() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/tasks/generate");
                then.status(400).body("bad task payload");
            })
            .await;

        let queue = ControlPlaneClient::new(
            HttpJsonProvider::new(server.base_url(), "test-key")
                .with_retry_policy(0, Duration::from_millis(1)),
        )
        .with_retry_policy(3, Duration::from_millis(1));
        queue.enqueue_generate("task_1", request());

        let err = queue.drain_once().await.unwrap_err();
        match err {
            ProviderError::HttpStatus { status, .. } => assert_eq!(status, 400),
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(queue.queued_tasks(), 0);
    }
}
