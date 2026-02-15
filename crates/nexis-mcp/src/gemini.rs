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

const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const DEFAULT_MODEL: &str = "gemini-1.5-flash";

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client should build"),
            api_key: api_key.into(),
            base_url: DEFAULT_GEMINI_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn generate_endpoint(&self, model: &str) -> String {
        format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            model,
            self.api_key
        )
    }

    fn stream_endpoint(&self, model: &str) -> String {
        format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url.trim_end_matches('/'),
            model,
            self.api_key
        )
    }

    fn payload(&self, req: GenerateRequest) -> (String, GeminiGenerateRequest) {
        let model = req.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
        (
            model,
            GeminiGenerateRequest {
                contents: vec![GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart { text: req.prompt }],
                }],
                generation_config: GeminiGenerationConfig {
                    max_output_tokens: req.max_tokens,
                    temperature: req.temperature,
                },
            },
        )
    }

    async fn parse_error_response(
        status: StatusCode,
        response: reqwest::Response,
    ) -> ProviderError {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unable to read body>".to_string());

        let parsed = serde_json::from_str::<GeminiErrorEnvelope>(&body)
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
impl AIProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        let (model, payload) = self.payload(req);

        let response = self
            .client
            .post(self.generate_endpoint(&model))
            .json(&payload)
            .send()
            .await
            .map_err(|err| ProviderError::Transport(err.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(Self::parse_error_response(status, response).await);
        }

        let body: GeminiGenerateResponse = response
            .json()
            .await
            .map_err(|err| ProviderError::Decode(err.to_string()))?;

        let first_candidate = body
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::Decode("missing candidate in response".to_string()))?;

        let content = first_candidate
            .content
            .parts
            .into_iter()
            .map(|part| part.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(GenerateResponse {
            content,
            model: Some(model),
            finish_reason: first_candidate.finish_reason,
        })
    }

    async fn generate_stream(&self, req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
        let (model, payload) = self.payload(req);
        let request = self.client.post(self.stream_endpoint(&model)).json(&payload);

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
                        let chunk = match serde_json::from_str::<GeminiGenerateResponse>(&message.data) {
                            Ok(chunk) => chunk,
                            Err(err) => {
                                let _ = tx.send(Err(ProviderError::Decode(err.to_string()))).await;
                                event_source.close();
                                break;
                            }
                        };

                        if let Some(candidate) = chunk.candidates.into_iter().next() {
                            for part in candidate.content.parts {
                                if !part.text.is_empty() {
                                    let _ = tx.send(Ok(StreamChunk::Delta { text: part.text })).await;
                                }
                            }

                            if candidate.finish_reason.is_some() {
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
struct GeminiGenerateRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct GeminiGenerateResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorEnvelope {
    error: GeminiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorDetail {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::GeminiProvider;
    use futures::StreamExt;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use nexis_runtime::{AIProvider, GenerateRequest, StreamChunk};
    use serde_json::json;

    fn request() -> GenerateRequest {
        GenerateRequest {
            prompt: "Say hello".to_string(),
            model: Some("gemini-1.5-flash".to_string()),
            max_tokens: Some(64),
            temperature: Some(0.1),
            metadata: None,
        }
    }

    #[tokio::test]
    async fn generate_calls_gemini_generate_content_endpoint() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1beta/models/gemini-1.5-flash:generateContent")
                    .query_param("key", "test-key");
                then.status(200).json_body(json!({
                    "candidates": [{
                        "content": {"parts": [{"text": "Hello from Gemini"}]},
                        "finishReason": "STOP"
                    }]
                }));
            })
            .await;

        let provider = GeminiProvider::new("test-key").with_base_url(server.base_url());
        let response = provider.generate(request()).await.unwrap();

        mock.assert_async().await;
        assert_eq!(response.content, "Hello from Gemini");
        assert_eq!(response.model.as_deref(), Some("gemini-1.5-flash"));
        assert_eq!(response.finish_reason.as_deref(), Some("STOP"));
    }

    #[tokio::test]
    async fn generate_stream_reads_sse_and_emits_delta_chunks() {
        let server = MockServer::start_async().await;
        let sse = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\"}]}\n\n"
        );

        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1beta/models/gemini-1.5-flash:streamGenerateContent")
                    .query_param("alt", "sse")
                    .query_param("key", "test-key");
                then.status(200)
                    .header("content-type", "text/event-stream")
                    .body(sse);
            })
            .await;

        let provider = GeminiProvider::new("test-key").with_base_url(server.base_url());
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
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1beta/models/gemini-1.5-flash:generateContent")
                    .query_param("key", "test-key");
                then.status(400).json_body(json!({
                    "error": {
                        "code": 400,
                        "message": "Invalid API key",
                        "status": "INVALID_ARGUMENT"
                    }
                }));
            })
            .await;

        let provider = GeminiProvider::new("test-key").with_base_url(server.base_url());
        let err = provider.generate(request()).await.unwrap_err();

        let display = err.to_string();
        assert!(display.contains("400"));
        assert!(display.contains("Invalid API key"));
    }
}
