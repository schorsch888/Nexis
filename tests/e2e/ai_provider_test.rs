use std::sync::Arc;

use httpmock::prelude::*;
use nexis_runtime::{AIProvider, GenerateRequest, OpenAIProvider, ProviderRegistry};

#[tokio::test]
#[ignore = "e2e-ish integration using httpmock"]
async fn ai_provider_end_to_end_with_mock_api() {
    let server = MockServer::start();

    let completion_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            .header("authorization", "Bearer test-api-key");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "id": "chatcmpl-mock-1",
                "object": "chat.completion",
                "created": 1700000000,
                "model": "gpt-4o-mini",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "mocked answer"
                        },
                        "finish_reason": "stop"
                    }
                ],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            }));
    });

    let provider = Arc::new(OpenAIProvider::new(
        "test-api-key",
        format!("{}/v1", server.base_url()),
        "gpt-4o-mini",
    ));

    let registry = ProviderRegistry::new();
    registry.register("openai", provider).await;

    let req = GenerateRequest {
        prompt: "Say mocked answer".to_string(),
        model: None,
        max_tokens: Some(32),
        temperature: Some(0.0),
        metadata: None,
    };

    let default_provider = registry
        .get_default()
        .await
        .expect("default provider should exist");
    assert_eq!(default_provider.name(), "openai");

    let response = default_provider
        .generate(req)
        .await
        .expect("provider generate should succeed");

    completion_mock.assert();
    assert_eq!(response.content, "mocked answer");
    assert_eq!(response.model.as_deref(), Some("gpt-4o-mini"));
    assert_eq!(response.finish_reason.as_deref(), Some("stop"));
}
