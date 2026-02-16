//! OpenAI API Provider
//!
//! Implements the AIProvider trait for OpenAI's Chat Completions API
//! with support for streaming responses.

use async_trait::async_trait;
use reqwest::Client;
use std::env;
use std::time::Duration;

use crate::{AIProvider, GenerateRequest, GenerateResponse, ProviderError, ProviderStream};

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
    
    async fn generate(&self, _req: GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        todo!("Implement generate")
    }
    
    async fn generate_stream(&self, _req: GenerateRequest) -> Result<ProviderStream, ProviderError> {
        todo!("Implement generate_stream")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
}
