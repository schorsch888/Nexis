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

const OPENAI_CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client should build"),
            api_key: api_key.into(),
            base_url: DEFAULT_OPENAI_BASE_URL.to_string(),
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
            OPENAI_CHAT_COMPLETIONS_PATH
        )
    }

    fn payload(&self, req: GenerateRequest, stream: bool) -> OpenAIChatCompletionRequest {
        OpenAIChatCompletionRequest {
            model: req.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            messages: vec![OpenAIChatMessage {
                role: "user".to_string(),
                content: req.prompt,
            }],
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            stream,
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

        let parsed = serde_json::from_str::<OpenAIErrorEnvelope>(&body)
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
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        let payload = self.payload(req, false);
        let response = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|err| ProviderError::Transport(err.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(Self::parse_error_response(status, response).await);
        }

        let body: OpenAIChatCompletionResponse = response
            .json()
            .await
            .map_err(|err| ProviderError::Decode(err.to_string()))?;

        let first_choice = body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::Decode("missing choice in response".to_string()))?;
        let content = first_choice.message.content.unwrap_or_default();

        Ok(GenerateResponse {
            content,
            model: Some(body.model),
            finish_reason: first_choice.finish_reason,
        })
    }

    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
        let payload = self.payload(req, true);
        let request = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
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
                        if message.data == "[DONE]" {
                            let _ = tx.send(Ok(StreamChunk::Done)).await;
                            done_sent = true;
                            event_source.close();
                            break;
                        }

                        let chunk = match serde_json::from_str::<OpenAIChatCompletionChunk>(&message.data)
                        {
                            Ok(chunk) => chunk,
                            Err(err) => {
                                let _ = tx
                                    .send(Err(ProviderError::Decode(err.to_string())))
                                    .await;
                                event_source.close();
                                break;
                            }
                        };

                        if let Some(choice) = chunk.choices.into_iter().next() {
                            if let Some(text) = choice.delta.content {
                                if !text.is_empty() {
                                    let _ = tx.send(Ok(StreamChunk::Delta { text })).await;
                                }
                            }

                            if choice.finish_reason.is_some() {
                                let _ = tx.send(Ok(StreamChunk::Done)).await;
                                done_sent = true;
                                event_source.close();
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(Err(ProviderError::Transport(err.to_string()))).await;
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
struct OpenAIChatCompletionRequest {
    model: String,
    messages: Vec<OpenAIChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OpenAIChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatCompletionResponse {
    model: String,
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatCompletionChunk {
    choices: Vec<OpenAIChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChunkChoice {
    delta: OpenAIChunkDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChunkDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorEnvelope {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::OpenAIProvider;
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
            model: Some("gpt-4o-mini".to_string()),
            max_tokens: Some(32),
            temperature: Some(0.2),
            metadata: None,
        }
    }

    #[tokio::test]
    async fn generate_calls_openai_chat_completions_endpoint() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1/chat/completions")
                    .body_includes("\"stream\":false");
                then.status(200).json_body(json!({
                    "id": "chatcmpl-1",
                    "object": "chat.completion",
                    "model": "gpt-4o-mini",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hello there"},
                        "finish_reason": "stop"
                    }]
                }));
            })
            .await;

        let provider = OpenAIProvider::new("test-key").with_base_url(server.base_url());
        let response = provider.generate(request()).await.unwrap();

        mock.assert_async().await;
        assert_eq!(response.content, "Hello there");
        assert_eq!(response.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    }

    #[tokio::test]
    async fn generate_stream_reads_sse_and_emits_delta_chunks() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start_async().await;
        let sse = concat!(
            "data: {\"id\":\"chatcmpl-2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hel\"}}]}\n\n",
            "data: {\"id\":\"chatcmpl-2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"}}]}\n\n",
            "data: {\"id\":\"chatcmpl-2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n"
        );

        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1/chat/completions")
                    .body_includes("\"stream\":true");
                then.status(200)
                    .header("content-type", "text/event-stream")
                    .body(sse);
            })
            .await;

        let provider = OpenAIProvider::new("test-key").with_base_url(server.base_url());
        let mut stream = provider.generate_stream(request()).await.unwrap();

        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        let done = stream.next().await.unwrap().unwrap();

        mock.assert_async().await;
        assert_eq!(first, StreamChunk::Delta { text: "Hel".to_string() });
        assert_eq!(second, StreamChunk::Delta { text: "lo".to_string() });
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
                when.method(POST).path("/v1/chat/completions");
                then.status(429).json_body(json!({
                    "error": {
                        "message": "Rate limit exceeded",
                        "type": "rate_limit_error"
                    }
                }));
            })
            .await;

        let provider = OpenAIProvider::new("test-key").with_base_url(server.base_url());
        let err = provider.generate(request()).await.unwrap_err();

        let display = err.to_string();
        assert!(display.contains("429"));
        assert!(display.contains("Rate limit exceeded"));
    }
}
