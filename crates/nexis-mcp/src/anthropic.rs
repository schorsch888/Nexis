use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use nexis_runtime::{
    AIProvider, GenerateRequest, GenerateResponse, ProviderError, ProviderStream, StreamChunk,
};
use reqwest::StatusCode;
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";
const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-3-5-haiku-latest";
const DEFAULT_MAX_TOKENS: u32 = 1024;

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    api_version: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client should build"),
            api_key: api_key.into(),
            base_url: DEFAULT_ANTHROPIC_BASE_URL.to_string(),
            api_version: DEFAULT_ANTHROPIC_VERSION.to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn endpoint(&self) -> String {
        format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            ANTHROPIC_MESSAGES_PATH
        )
    }

    fn payload(&self, req: GenerateRequest, stream: bool) -> AnthropicMessageRequest {
        AnthropicMessageRequest {
            model: req.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens: req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            temperature: req.temperature,
            stream,
            messages: vec![AnthropicInputMessage {
                role: "user".to_string(),
                content: vec![AnthropicInputBlock {
                    kind: "text".to_string(),
                    text: req.prompt,
                }],
            }],
        }
    }

    async fn parse_error_response(
        status: StatusCode,
        response: reqwest::Response,
    ) -> ProviderError {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unable to read body>".to_string());

        let parsed = serde_json::from_str::<AnthropicErrorEnvelope>(&body)
            .ok()
            .map(|err| err.error.message)
            .unwrap_or_else(|| body.clone());

        ProviderError::HttpStatus {
            status: status.as_u16(),
            body: parsed,
        }
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        let payload = self.payload(req, false);
        let response = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .json(&payload)
            .send()
            .await
            .map_err(|err| ProviderError::Transport(err.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(Self::parse_error_response(status, response).await);
        }

        let body: AnthropicMessageResponse = response
            .json()
            .await
            .map_err(|err| ProviderError::Decode(err.to_string()))?;

        let content = body
            .content
            .iter()
            .filter_map(|block| block.text.as_deref())
            .collect::<Vec<_>>()
            .join("");

        Ok(GenerateResponse {
            content,
            model: Some(body.model),
            finish_reason: body.stop_reason,
        })
    }

    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
        let payload = self.payload(req, true);
        let request = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .json(&payload);

        let mut event_source = request
            .eventsource()
            .map_err(|err| ProviderError::Transport(err.to_string()))?;

        let (tx, rx) = mpsc::channel::<Result<StreamChunk, ProviderError>>(32);
        tokio::spawn(async move {
            let mut done_sent = false;

            while let Some(event) = event_source.next().await {
                match event {
                    Ok(Event::Open) => continue,
                    Ok(Event::Message(message)) => {
                        if message.event == "message_stop" {
                            let _ = tx.send(Ok(StreamChunk::Done)).await;
                            done_sent = true;
                            event_source.close();
                            break;
                        }

                        if message.event == "error" {
                            match serde_json::from_str::<AnthropicErrorEnvelope>(&message.data) {
                                Ok(err) => {
                                    let _ = tx
                                        .send(Err(ProviderError::Message(err.error.message)))
                                        .await;
                                }
                                Err(err) => {
                                    let _ =
                                        tx.send(Err(ProviderError::Decode(err.to_string()))).await;
                                }
                            }
                            event_source.close();
                            break;
                        }

                        if message.event == "content_block_delta" {
                            let delta =
                                match serde_json::from_str::<AnthropicStreamDelta>(&message.data) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        let _ = tx
                                            .send(Err(ProviderError::Decode(err.to_string())))
                                            .await;
                                        event_source.close();
                                        break;
                                    }
                                };

                            if let Some(text) = delta.delta.text {
                                if !text.is_empty() {
                                    let _ = tx.send(Ok(StreamChunk::Delta { text })).await;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        let _ = tx
                            .send(Err(ProviderError::Transport(err.to_string())))
                            .await;
                        event_source.close();
                        break;
                    }
                }
            }

            if !done_sent {
                let _ = tx.send(Ok(StreamChunk::Done)).await;
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

#[derive(Debug, Serialize)]
struct AnthropicMessageRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
    messages: Vec<AnthropicInputMessage>,
}

#[derive(Debug, Serialize)]
struct AnthropicInputMessage {
    role: String,
    content: Vec<AnthropicInputBlock>,
}

#[derive(Debug, Serialize)]
struct AnthropicInputBlock {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageResponse {
    model: String,
    content: Vec<AnthropicContentBlock>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamDelta {
    delta: AnthropicDeltaText,
}

#[derive(Debug, Deserialize)]
struct AnthropicDeltaText {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorEnvelope {
    error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::AnthropicProvider;
    use futures::StreamExt;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use nexis_runtime::{AIProvider, GenerateRequest, StreamChunk};
    use serde_json::json;

    fn network_tests_enabled() -> bool {
        matches!(std::env::var("NEXIS_RUN_NETWORK_TESTS"), Ok(value) if value == "1")
    }

    fn request() -> GenerateRequest {
        GenerateRequest {
            prompt: "Say hello".to_string(),
            model: Some("claude-3-5-haiku-latest".to_string()),
            max_tokens: Some(64),
            temperature: Some(0.1),
            metadata: None,
        }
    }

    #[tokio::test]
    async fn generate_calls_anthropic_messages_endpoint() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1/messages")
                    .header("x-api-key", "test-key")
                    .header("anthropic-version", "2023-06-01")
                    .body_includes("\"stream\":false");
                then.status(200).json_body(json!({
                    "id": "msg_1",
                    "type": "message",
                    "model": "claude-3-5-haiku-latest",
                    "stop_reason": "end_turn",
                    "content": [{"type":"text","text":"Hello from Claude"}]
                }));
            })
            .await;

        let provider = AnthropicProvider::new("test-key").with_base_url(server.base_url());
        let response = provider.generate(request()).await.unwrap();

        mock.assert_async().await;
        assert_eq!(response.content, "Hello from Claude");
        assert_eq!(response.model.as_deref(), Some("claude-3-5-haiku-latest"));
        assert_eq!(response.finish_reason.as_deref(), Some("end_turn"));
    }

    #[tokio::test]
    async fn generate_stream_reads_sse_and_emits_delta_chunks() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start_async().await;
        let sse = concat!(
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"lo\"}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n"
        );

        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1/messages")
                    .body_includes("\"stream\":true");
                then.status(200)
                    .header("content-type", "text/event-stream")
                    .body(sse);
            })
            .await;

        let provider = AnthropicProvider::new("test-key").with_base_url(server.base_url());
        let mut stream = provider.generate_stream(request()).await.unwrap();

        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        let done = stream.next().await.unwrap().unwrap();

        mock.assert_async().await;
        assert_eq!(
            first,
            StreamChunk::Delta {
                text: "Hel".to_string()
            }
        );
        assert_eq!(
            second,
            StreamChunk::Delta {
                text: "lo".to_string()
            }
        );
        assert_eq!(done, StreamChunk::Done);
    }

    #[tokio::test]
    async fn generate_maps_non_success_status_to_provider_error() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/messages");
                then.status(400).json_body(json!({
                    "type":"error",
                    "error": {
                        "type": "invalid_request_error",
                        "message": "messages is required"
                    }
                }));
            })
            .await;

        let provider = AnthropicProvider::new("test-key").with_base_url(server.base_url());
        let err = provider.generate(request()).await.unwrap_err();

        let display = err.to_string();
        assert!(display.contains("400"));
        assert!(display.contains("messages is required"));
    }
}
