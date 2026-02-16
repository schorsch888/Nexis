//! OpenAI API Provider
//!
//! Implements the AIProvider trait for OpenAI's Chat Completions API
//! with support for streaming responses.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use crate::{AIProvider, GenerateRequest, GenerateResponse, ProviderError, ProviderStream, StreamChunk};
use futures::StreamExt;

const OPENAI_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

#[derive(Debug)]
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    pub default_model: String,
}

impl OpenAIProvider {
    pub fn from_env() -> Self {
        let api_key = env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable must be set");
        
        let base_url = env::var("OPENAI_API_BASE")
            .unwrap_or_else(|_| OPENAI_API_BASE.to_string());
        
        let default_model = env::var("OPENAI_DEFAULT_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        
        Self::new(api_key, base_url, default_model)
    }
    
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
    
    pub fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
    
    pub fn get_model(&self, req: &GenerateRequest) -> String {
        req.model.clone().unwrap_or_else(|| self.default_model.clone())
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }
    
    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        let openai_req = ChatCompletionRequest {
            model: self.get_model(&req),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: req.prompt,
                }
            ],
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            stream: None,
        };

        let response = self.client
            .post(self.endpoint("/chat/completions"))
            .bearer_auth(&self.api_key)
            .json(&openai_req)
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

        let openai_resp: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Decode(e.to_string()))?;

        // Extract the assistant's message
        let content = openai_resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| ProviderError::Decode("No choices in response".to_string()))?;

        Ok(GenerateResponse {
            content,
            model: Some(openai_resp.model),
            finish_reason: openai_resp.choices.first().and_then(|c| c.finish_reason.clone()),
        })
    }
    
    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
        use futures::stream;
        use reqwest_eventsource::{Event, EventSource};
        
        let openai_req = ChatCompletionRequest {
            model: self.get_model(&req),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: req.prompt,
                }
            ],
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            stream: Some(true),
        };
        
        let client = self.client.clone();
        let endpoint = self.endpoint("/chat/completions");
        let api_key = self.api_key.clone();
        
        // Create EventSource for SSE streaming
        let event_source = EventSource::new(
            client
                .post(&endpoint)
                .bearer_auth(&api_key)
                .json(&openai_req)
        )
        .map_err(|e| ProviderError::Transport(e.to_string()))?;
        
        // Convert EventSource to Stream<StreamChunk>
        let stream = event_source
            .take_while(|event| {
                futures::future::ready(!matches!(event, Ok(Event::Message(ref msg)) if msg.data == "[DONE]"))
            })
            .filter_map(|event| async move {
                match event {
                    Ok(Event::Message(msg)) => {
                        // Parse the chunk
                        let chunk: Result<ChatCompletionChunk, _> = serde_json::from_str(&msg.data);
                        
                        match chunk {
                            Ok(chunk) => {
                                // Extract text from delta
                                let text = chunk.choices.first()
                                    .and_then(|c| c.delta.content.clone())
                                    .unwrap_or_default();
                                
                                if text.is_empty() {
                                    None
                                } else {
                                    Some(Ok(StreamChunk::Delta { text }))
                                }
                            }
                            Err(e) => Some(Err(ProviderError::Decode(e.to_string()))),
                        }
                    }
                    Ok(Event::Open) => None,
                    Err(e) => Some(Err(ProviderError::Transport(e.to_string()))),
                }
            })
            .chain(stream::iter(vec![Ok(StreamChunk::Done)]));
        
        Ok(Box::pin(stream))
    }
}

// ============================================================================
// OpenAI API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    index: u32,
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    index: u32,
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StreamChunk;
    use httpmock::prelude::*;
    use serde_json::json;
    use futures::StreamExt;
    
    #[test]
    fn provider_creation_explicit() {
        let provider = OpenAIProvider::new(
            "test-key",
            "https://api.example.com/v1",
            "gpt-4"
        );
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model, "gpt-4");
    }
    
    #[test]
    fn endpoint_building() {
        let provider = OpenAIProvider::new("key", "https://api.openai.com/v1", "gpt-4");
        assert_eq!(provider.endpoint("/chat/completions"), "https://api.openai.com/v1/chat/completions");
        
        let provider2 = OpenAIProvider::new("key", "https://api.openai.com/v1/", "gpt-4");
        assert_eq!(provider2.endpoint("/chat/completions"), "https://api.openai.com/v1/chat/completions");
    }
    
    #[test]
    fn get_model_uses_default_when_not_specified() {
        let provider = OpenAIProvider::new("key", "https://api.example.com/v1", "gpt-4-turbo");
        
        let req = GenerateRequest {
            prompt: "test".to_string(),
            model: None,
            max_tokens: None,
            temperature: None,
            metadata: None,
        };
        
        assert_eq!(provider.get_model(&req), "gpt-4-turbo");
    }
    
    #[test]
    fn get_model_uses_request_model_when_specified() {
        let provider = OpenAIProvider::new("key", "https://api.example.com/v1", "gpt-4-turbo");
        
        let req = GenerateRequest {
            prompt: "test".to_string(),
            model: Some("gpt-3.5-turbo".to_string()),
            max_tokens: None,
            temperature: None,
            metadata: None,
        };
        
        assert_eq!(provider.get_model(&req), "gpt-3.5-turbo");
    }
    
    #[test]
    fn provider_creation_from_env() {
        if env::var("OPENAI_API_KEY").is_ok() {
            let provider = OpenAIProvider::from_env();
            assert_eq!(provider.name(), "openai");
        }
    }
    
    #[test]
    fn chat_completion_request_serialization() {
        let req = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                }
            ],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: None,
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\":\"gpt-4\""));
        assert!(json.contains("\"max_tokens\":100"));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(!json.contains("\"stream\""));
    }
    
    #[test]
    fn chat_completion_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello there!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;
        
        let resp: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "chatcmpl-123");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "Hello there!");
        assert_eq!(resp.usage.unwrap().total_tokens, 15);
    }
    
    #[tokio::test]
    async fn generate_calls_openai_api() {
        let server = MockServer::start();
        
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .header("Authorization", "Bearer test-key");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "id": "chatcmpl-test",
                    "object": "chat.completion",
                    "created": 1234567890,
                    "model": "gpt-4",
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Hello! How can I help you?"
                        },
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 10,
                        "completion_tokens": 20,
                        "total_tokens": 30
                    }
                }));
        });
        
        let provider = OpenAIProvider::new(
            "test-key",
            server.base_url(),
            "gpt-4"
        );
        
        let req = GenerateRequest {
            prompt: "Hello".to_string(),
            model: None,
            max_tokens: Some(100),
            temperature: Some(0.7),
            metadata: None,
        };
        
        let resp = provider.generate(req).await.unwrap();
        
        mock.assert();
        assert_eq!(resp.content, "Hello! How can I help you?");
        assert_eq!(resp.model, Some("gpt-4".to_string()));
        assert_eq!(resp.finish_reason, Some("stop".to_string()));
    }
    
    #[tokio::test]
    async fn generate_handles_api_error() {
        let server = MockServer::start();
        
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(401)
                .json_body(json!({
                    "error": {
                        "message": "Invalid API key",
                        "type": "invalid_request_error"
                    }
                }));
        });
        
        let provider = OpenAIProvider::new("bad-key", server.base_url(), "gpt-4");
        
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
    
    #[tokio::test]
    async fn generate_stream_emits_chunks() {
        use futures::StreamExt;
        
        let server = MockServer::start();
        
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .header("Authorization", "Bearer test-key");
            then.status(200)
                .header("content-type", "text/event-stream")
                .body(concat!(
                    "data: {\"id\":\"chatcmpl-test\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
                    "data: {\"id\":\"chatcmpl-test\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
                    "data: {\"id\":\"chatcmpl-test\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"!\"},\"finish_reason\":null}]}\n\n",
                    "data: [DONE]\n\n"
                ));
        });
        
        let provider = OpenAIProvider::new("test-key", server.base_url(), "gpt-4");
        
        let req = GenerateRequest {
            prompt: "Hi".to_string(),
            model: None,
            max_tokens: None,
            temperature: None,
            metadata: None,
        };
        
        let mut stream = provider.generate_stream(req).await.unwrap();
        
        let mut chunks = vec![];
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.unwrap());
        }
        
        mock.assert();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], StreamChunk::Delta { text: "Hello".to_string() });
        assert_eq!(chunks[1], StreamChunk::Delta { text: "!".to_string() });
        assert_eq!(chunks[2], StreamChunk::Done);
    }
}
