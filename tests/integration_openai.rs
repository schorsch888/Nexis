//! Integration tests for OpenAI Provider
//!
//! These tests require OPENAI_API_KEY to be set
//! Run with: cargo test --test integration_openai -- --ignored

use futures::StreamExt;
use nexis_runtime::{AIProvider, GenerateRequest, OpenAIProvider, StreamChunk};

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn openai_generate_real_api() {
    let provider = OpenAIProvider::from_env();

    let req = GenerateRequest {
        prompt: "Say 'Hello, Nexis!' and nothing else.".to_string(),
        model: Some("gpt-4o-mini".to_string()),
        max_tokens: Some(50),
        temperature: Some(0.0),
        metadata: None,
    };

    let resp = provider.generate(req).await.unwrap();

    assert!(resp.content.contains("Hello"));
    assert_eq!(resp.model, Some("gpt-4o-mini".to_string()));
    println!("Response: {}", resp.content);
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn openai_stream_real_api() {
    let provider = OpenAIProvider::from_env();

    let req = GenerateRequest {
        prompt: "Count from 1 to 5, one number per line.".to_string(),
        model: Some("gpt-4o-mini".to_string()),
        max_tokens: Some(50),
        temperature: Some(0.0),
        metadata: None,
    };

    let mut stream = provider.generate_stream(req).await.unwrap();

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk.unwrap() {
            StreamChunk::Delta { text } => {
                full_text.push_str(&text);
                print!("{}", text);
            }
            StreamChunk::Done => break,
        }
    }

    println!("\nFull response: {}", full_text);
    assert!(full_text.contains('1'));
    assert!(full_text.contains('5'));
}
