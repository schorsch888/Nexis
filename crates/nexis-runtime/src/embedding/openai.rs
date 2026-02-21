//! OpenAI Embedding Provider
//!
//! Implements the EmbeddingProvider trait for OpenAI's Embeddings API

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use crate::embedding::{BatchEmbeddingRequest, BatchEmbeddingResponse, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage, DEFAULT_EMBEDDING_DIMENSION};
use crate::ProviderError;

const OPENAI_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "text-embedding-3-small";

#[derive(Debug)]
pub struct OpenAIEmbeddingProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
    dimension: usize,
    max_retries: u32,
    retry_base_delay: Duration,
}

impl OpenAIEmbeddingProvider {
    pub fn from_env() -> Self {
        let api_key =
            env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable must be set");

        let base_url = env::var("OPENAI_API_BASE").unwrap_or_else(|_| OPENAI_API_BASE.to_string());

        let default_model =
            env::var("OPENAI_EMBEDDING_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        let dimension = match default_model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => DEFAULT_EMBEDDING_DIMENSION,
        };

        Self::new(api_key, base_url, default_model, dimension)
    }

    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        default_model: impl Into<String>,
        dimension: usize,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
            default_model: default_model.into(),
            dimension,
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

    fn get_model(&self, req_model: Option<&String>) -> String {
        req_model
            .cloned()
            .unwrap_or_else(|| self.default_model.clone())
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequestBody {
    input: EmbeddingInput,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum EmbeddingInput {
    Single(String),
    Batch(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingResponseBody {
    object: String,
    data: Vec<EmbeddingData>,
    model: String,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingData {
    object: String,
    index: u32,
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

fn is_retriable(err: &ProviderError) -> bool {
    match err {
        ProviderError::Transport(_) => true,
        ProviderError::HttpStatus { status, .. } => *status >= 500 || *status == 429,
        _ => false,
    }
}

fn backoff(base: Duration, attempt: u32) -> Duration {
    base.saturating_mul(1_u32 << attempt)
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    fn name(&self) -> &'static str {
        "openai-embedding"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn embed(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, ProviderError> {
        let model = self.get_model(req.model.as_ref());
        let body = EmbeddingRequestBody {
            input: EmbeddingInput::Single(req.text),
            model: model.clone(),
            dimensions: None,
        };

        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            match self.try_embed(&body).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    let retriable = is_retriable(&err);
                    last_error = Some(err.to_string());
                    if retriable && attempt < self.max_retries {
                        tokio::time::sleep(backoff(self.retry_base_delay, attempt)).await;
                        continue;
                    }
                    if retriable {
                        return Err(ProviderError::RetryExhausted {
                            attempts: attempt + 1,
                            last_error: last_error.unwrap_or_else(|| "unknown retry error".to_string()),
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

    async fn embed_batch(&self, req: BatchEmbeddingRequest) -> Result<BatchEmbeddingResponse, ProviderError> {
        if req.texts.is_empty() {
            return Ok(BatchEmbeddingResponse {
                embeddings: Vec::new(),
                model: self.default_model.clone(),
                dimension: self.dimension,
                usage: None,
            });
        }

        let model = self.get_model(req.model.as_ref());
        let body = EmbeddingRequestBody {
            input: EmbeddingInput::Batch(req.texts),
            model: model.clone(),
            dimensions: None,
        };

        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            match self.try_embed_batch(&body).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    let retriable = is_retriable(&err);
                    last_error = Some(err.to_string());
                    if retriable && attempt < self.max_retries {
                        tokio::time::sleep(backoff(self.retry_base_delay, attempt)).await;
                        continue;
                    }
                    if retriable {
                        return Err(ProviderError::RetryExhausted {
                            attempts: attempt + 1,
                            last_error: last_error.unwrap_or_else(|| "unknown retry error".to_string()),
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
}

impl OpenAIEmbeddingProvider {
    async fn try_embed(&self, body: &EmbeddingRequestBody) -> Result<EmbeddingResponse, ProviderError> {
        let response = self
            .client
            .post(self.endpoint("/embeddings"))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

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

        let resp: EmbeddingResponseBody = response
            .json()
            .await
            .map_err(|e| ProviderError::Decode(e.to_string()))?;

        let data = resp.data.first().ok_or_else(|| {
            ProviderError::Decode("No embedding data in response".to_string())
        })?;

        Ok(EmbeddingResponse {
            embedding: data.embedding.clone(),
            model: resp.model,
            dimension: data.embedding.len(),
            usage: Some(EmbeddingUsage::new(
                resp.usage.prompt_tokens,
                resp.usage.total_tokens,
            )),
        })
    }

    async fn try_embed_batch(&self, body: &EmbeddingRequestBody) -> Result<BatchEmbeddingResponse, ProviderError> {
        let response = self
            .client
            .post(self.endpoint("/embeddings"))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

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

        let resp: EmbeddingResponseBody = response
            .json()
            .await
            .map_err(|e| ProviderError::Decode(e.to_string()))?;

        let mut embeddings: Vec<(u32, Vec<f32>)> = resp
            .data
            .iter()
            .map(|d| (d.index, d.embedding.clone()))
            .collect();
        embeddings.sort_by_key(|(idx, _)| *idx);

        Ok(BatchEmbeddingResponse {
            embeddings: embeddings.into_iter().map(|(_, emb)| emb).collect(),
            model: resp.model,
            dimension: self.dimension,
            usage: Some(EmbeddingUsage::new(
                resp.usage.prompt_tokens,
                resp.usage.total_tokens,
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_creation() {
        let provider = OpenAIEmbeddingProvider::new(
            "test-key",
            "https://api.openai.com/v1",
            "text-embedding-3-small",
            1536,
        );
        assert_eq!(provider.name(), "openai-embedding");
        assert_eq!(provider.dimension(), 1536);
    }

    #[test]
    fn get_model_uses_default_when_not_specified() {
        let provider = OpenAIEmbeddingProvider::new(
            "key",
            "https://api.example.com/v1",
            "text-embedding-3-large",
            3072,
        );

        let model = provider.get_model(None);
        assert_eq!(model, "text-embedding-3-large");
    }

    #[test]
    fn get_model_uses_request_model_when_specified() {
        let provider = OpenAIEmbeddingProvider::new(
            "key",
            "https://api.example.com/v1",
            "text-embedding-3-small",
            1536,
        );

        let model = provider.get_model(Some(&"text-embedding-ada-002".to_string()));
        assert_eq!(model, "text-embedding-ada-002");
    }

    #[test]
    fn request_body_serialization_single() {
        let body = EmbeddingRequestBody {
            input: EmbeddingInput::Single("hello world".to_string()),
            model: "text-embedding-3-small".to_string(),
            dimensions: None,
        };

        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"input\":\"hello world\""));
        assert!(json.contains("\"model\":\"text-embedding-3-small\""));
    }

    #[test]
    fn request_body_serialization_batch() {
        let body = EmbeddingRequestBody {
            input: EmbeddingInput::Batch(vec!["hello".to_string(), "world".to_string()]),
            model: "text-embedding-3-small".to_string(),
            dimensions: None,
        };

        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"input\":[\"hello\",\"world\"]"));
    }

    #[test]
    fn response_deserialization() {
        let json = r#"{
            "object": "list",
            "data": [{
                "object": "embedding",
                "index": 0,
                "embedding": [0.1, 0.2, 0.3]
            }],
            "model": "text-embedding-3-small",
            "usage": {
                "prompt_tokens": 5,
                "total_tokens": 5
            }
        }"#;

        let resp: EmbeddingResponseBody = serde_json::from_str(json).unwrap();
        assert_eq!(resp.model, "text-embedding-3-small");
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].embedding, vec![0.1, 0.2, 0.3]);
        assert_eq!(resp.usage.prompt_tokens, 5);
    }

    fn network_tests_enabled() -> bool {
        matches!(std::env::var("NEXIS_RUN_NETWORK_TESTS"), Ok(value) if value == "1")
    }

    #[tokio::test]
    async fn embed_calls_openai_api() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        use httpmock::prelude::*;

        let server = MockServer::start();

        // Create a small test embedding response
        let embedding_array: Vec<String> = (0..10).map(|_| "0.1".to_string()).collect();
        let response_body = format!(r#"{{
            "object": "list",
            "data": [{{
                "object": "embedding",
                "index": 0,
                "embedding": [{}]
            }}],
            "model": "text-embedding-3-small",
            "usage": {{
                "prompt_tokens": 5,
                "total_tokens": 5
            }}
        }}"#, embedding_array.join(","));

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/embeddings")
                .header("Authorization", "Bearer test-key");
            then.status(200).body(&response_body);
        });

        let provider = OpenAIEmbeddingProvider::new(
            "test-key",
            server.base_url(),
            "text-embedding-3-small",
            1536,
        );

        let req = EmbeddingRequest::new("hello world");
        let resp = provider.embed(req).await.unwrap();

        mock.assert();
        assert_eq!(resp.model, "text-embedding-3-small");
        assert_eq!(resp.embedding.len(), 1536);
    }

    #[tokio::test]
    async fn embed_batch_orders_results_by_index() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        use httpmock::prelude::*;

        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/embeddings");
            then.status(200).json_body(serde_json::json!({
                "object": "list",
                "data": [
                    {"object": "embedding", "index": 1, "embedding": [0.2, 0.2]},
                    {"object": "embedding", "index": 0, "embedding": [0.1, 0.1]},
                    {"object": "embedding", "index": 2, "embedding": [0.3, 0.3]}
                ],
                "model": "text-embedding-3-small",
                "usage": {"prompt_tokens": 15, "total_tokens": 15}
            }));
        });

        let provider = OpenAIEmbeddingProvider::new(
            "test-key",
            server.base_url(),
            "text-embedding-3-small",
            2,
        );

        let req = BatchEmbeddingRequest::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let resp = provider.embed_batch(req).await.unwrap();

        mock.assert();
        assert_eq!(resp.embeddings.len(), 3);
        assert_eq!(resp.embeddings[0], vec![0.1, 0.1]);
        assert_eq!(resp.embeddings[1], vec![0.2, 0.2]);
        assert_eq!(resp.embeddings[2], vec![0.3, 0.3]);
    }
}
