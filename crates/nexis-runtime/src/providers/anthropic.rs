//! Anthropic Claude API Provider
//!
//! Implements the AIProvider trait for Anthropic's Messages API
//! with support for streaming responses.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use crate::{AIProvider, GenerateRequest, GenerateResponse, ProviderError, ProviderStream};

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const DEFAULT_MODEL: &str = "claude-3-5-sonnet-20241022";
const API_VERSION: &str = "2023-06-01";

/// Anthropic API Provider
#[derive(Debug)]
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl AnthropicProvider {
    /// Create new Anthropic provider from environment variable
    pub fn from_env() -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable must be set");
        
        let base_url = env::var("ANTHROPIC_API_BASE")
            .unwrap_or_else(|_| ANTHROPIC_API_BASE.to_string());
        
        let default_model = env::var("ANTHROPIC_DEFAULT_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        
        Self::new(api_key, base_url, default_model)
    }
    
    /// Create new Anthropic provider with explicit configuration
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>, default_model: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
            default_model: default_model.into(),
        }
    }
    
    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
    
    fn get_model(&self, req: &GenerateRequest) -> String {
        req.model.clone().unwrap_or_else(|| self.default_model.clone())
    }
}

// ============================================================================
// Anthropic API Types
// ============================================================================

/// Anthropic Messages Request
#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// Anthropic Message
#[derive(Debug, Serialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic Messages Response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessagesResponse {
    id: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    response_type: String,
    #[allow(dead_code)]
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    #[allow(dead_code)]
    usage: Usage,
}

/// Anthropic Content Block
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    block_type: String,
    text: String,
}

/// Anthropic Usage
#[derive(Debug, Deserialize)]
struct Usage {
    #[allow(dead_code)]
    input_tokens: u32,
    #[allow(dead_code)]
    output_tokens: u32,
}

/// Anthropic Streaming Event
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum StreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: DeltaContent,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
}

/// Anthropic Delta Content
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeltaContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: String,
    text: String,
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }
    
    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        let anthropic_req = MessagesRequest {
            model: self.get_model(&req),
            messages: vec![
                AnthropicMessage {
                    role: "user".to_string(),
                    content: req.prompt,
                }
            ],
            max_tokens: req.max_tokens.unwrap_or(1024),
            stream: None,
        };
        
        let response = self.client
            .post(self.endpoint("/messages"))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;
        
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| "<unable to read body>".to_string());
            return Err(ProviderError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }
        
        let anthropic_resp: MessagesResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Decode(e.to_string()))?;
        
        // Extract text from content blocks
        let content = anthropic_resp
            .content
            .iter()
            .filter_map(|block| {
                if block.block_type == "text" {
                    Some(block.text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");
        
        Ok(GenerateResponse {
            content,
            model: Some(anthropic_resp.model),
            finish_reason: anthropic_resp.stop_reason,
        })
    }
    
    async fn generate_stream(&self, _req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
        // TODO: Implement streaming for Anthropic
        // Anthropic uses different SSE format than OpenAI
        unimplemented!("Anthropic streaming not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    fn network_tests_enabled() -> bool {
        matches!(std::env::var("NEXIS_RUN_NETWORK_TESTS"), Ok(value) if value == "1")
    }
    
    #[test]
    fn provider_creation_explicit() {
        let provider = AnthropicProvider::new(
            "test-key",
            "https://api.anthropic.com/v1",
            "claude-3-opus"
        );
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.default_model, "claude-3-opus");
    }
    
    #[test]
    fn provider_creation_from_env() {
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            let provider = AnthropicProvider::from_env();
            assert_eq!(provider.name(), "anthropic");
        }
    }
    
    #[tokio::test]
    async fn generate_calls_anthropic_api() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start();
        
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/messages")
                .header("x-api-key", "test-key")
                .header("anthropic-version", API_VERSION);
            then.status(200)
                .json_body(json!({
                    "id": "msg_test",
                    "type": "message",
                    "role": "assistant",
                    "content": [{
                        "type": "text",
                        "text": "Hello! I'm Claude."
                    }],
                    "model": "claude-3-5-sonnet-20241022",
                    "stop_reason": "end_turn",
                    "usage": {
                        "input_tokens": 10,
                        "output_tokens": 20
                    }
                }));
        });
        
        let provider = AnthropicProvider::new("test-key", server.base_url(), "claude-3-5-sonnet-20241022");
        
        let req = GenerateRequest {
            prompt: "Hello".to_string(),
            model: None,
            max_tokens: Some(100),
            temperature: None,
            metadata: None,
        };
        
        let resp = provider.generate(req).await.unwrap();
        
        mock.assert();
        assert_eq!(resp.content, "Hello! I'm Claude.");
        assert_eq!(resp.model, Some("claude-3-5-sonnet-20241022".to_string()));
    }
    
    #[tokio::test]
    async fn generate_handles_api_error() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start();
        
        server.mock(|when, then| {
            when.method(POST).path("/messages");
            then.status(401)
                .json_body(json!({
                    "error": {
                        "type": "authentication_error",
                        "message": "Invalid API Key"
                    }
                }));
        });
        
        let provider = AnthropicProvider::new("bad-key", server.base_url(), "claude-3-5-sonnet-20241022");
        
        let req = GenerateRequest {
            prompt: "Hello".to_string(),
            model: None,
            max_tokens: None,
            temperature: None,
            metadata: None,
        };
        
        let err = provider.generate(req).await.unwrap_err();
        
        match err {
            ProviderError::HttpStatus { status, .. } => assert_eq!(status, 401),
            _ => panic!("Expected HttpStatus error"),
        }
    }
}
