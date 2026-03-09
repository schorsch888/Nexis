//! AI-powered context summarization
//!
//! This module provides an AI-backed summarizer that uses nexis-runtime
//! to generate conversation summaries when the context window overflows.
//!
//! Requires `ai-summarizer` feature to be enabled.

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{debug, warn, instrument};

use crate::context::Message;
use crate::error::{ContextError, ContextResult};
use crate::summarizer::{ContextSummarizer, SummarizerConfig};

#[cfg(feature = "ai-summarizer")]
use nexis_runtime::{AIProvider, GenerateRequest};

/// AI-powered summarizer using nexis-runtime providers
#[cfg(feature = "ai-summarizer")]
#[derive(Debug)]
pub struct AISummarizer {
    provider: Arc<dyn AIProvider>,
    model: String,
    config: SummarizerConfig,
}

#[cfg(feature = "ai-summarizer")]
impl AISummarizer {
    /// Create a new AI summarizer with the given provider
    pub fn new(provider: Arc<dyn AIProvider>, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            config: SummarizerConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        provider: Arc<dyn AIProvider>,
        model: impl Into<String>,
        config: SummarizerConfig,
    ) -> Self {
        Self {
            provider,
            model: model.into(),
            config,
        }
    }

    /// Build the summarization prompt from messages
    fn build_prompt(&self, messages: &[Message]) -> String {
        let formatted = self.config.format_messages(messages);
        format!(
            "You are a conversation summarizer. Create a concise summary that preserves:\n\
             - Key decisions and conclusions\n\
             - Important facts and context\n\
             - Action items or next steps\n\
             - Open questions or unresolved issues\n\n\
             Be brief but comprehensive. Use bullet points if helpful.\n\n\
             {}",
            formatted
        )
    }
}

#[cfg(feature = "ai-summarizer")]
#[async_trait]
impl ContextSummarizer for AISummarizer {
    #[instrument(skip(self, messages), fields(message_count = messages.len()))]
    async fn summarize(&self, messages: &[Message]) -> ContextResult<Message> {
        if messages.is_empty() {
            return Err(ContextError::InvalidMessage(
                "Cannot summarize empty message list".to_string(),
            ));
        }

        let prompt = self.build_prompt(messages);
        debug!(message_count = messages.len(), "Starting AI summarization");

        let request = GenerateRequest {
            prompt,
            model: Some(self.model.clone()),
            max_tokens: Some(self.config.max_summary_tokens as u32),
            temperature: Some(0.3), // Lower temperature for more consistent summaries
            metadata: None,
        };

        match self.provider.generate(request).await {
            Ok(response) => {
                debug!(
                    summary_len = response.content.len(),
                    "AI summarization complete"
                );
                
                let mut summary = Message::system(format!(
                    "[Summary of {} messages]\n{}",
                    messages.len(),
                    response.content
                ));
                // Estimate token count for the summary
                summary.token_count = Some(response.content.len() / 4);
                
                Ok(summary)
            }
            Err(e) => {
                warn!(error = %e, "AI summarization failed");
                Err(ContextError::SummarizationFailed(format!(
                    "AI provider error: {}",
                    e
                )))
            }
        }
    }
}

// When ai-summarizer feature is not enabled, provide a stub
#[cfg(not(feature = "ai-summarizer"))]
#[derive(Debug, Clone)]
pub struct AISummarizer;

#[cfg(not(feature = "ai-summarizer"))]
impl AISummarizer {
    pub fn new_stub() -> Self {
        Self
    }
}

#[cfg(not(feature = "ai-summarizer"))]
#[async_trait]
impl ContextSummarizer for AISummarizer {
    async fn summarize(&self, _messages: &[Message]) -> ContextResult<Message> {
        Err(ContextError::SummarizationNotAvailable)
    }
}

#[cfg(all(test, feature = "ai-summarizer"))]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockAIProvider {
        response: String,
    }

    #[async_trait]
    impl AIProvider for MockAIProvider {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn generate(
            &self,
            _req: GenerateRequest,
        ) -> Result<nexis_runtime::GenerateResponse, nexis_runtime::ProviderError> {
            Ok(nexis_runtime::GenerateResponse {
                content: self.response.clone(),
                model: Some("mock-model".to_string()),
                finish_reason: Some("stop".to_string()),
            })
        }

        async fn generate_stream(
            &self,
            _req: GenerateRequest,
        ) -> Result<nexis_runtime::ProviderStream, nexis_runtime::ProviderError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_ai_summarizer_generates_summary() {
        let provider = Arc::new(MockAIProvider {
            response: "User asked about the project status. Assistant provided updates on three features.".to_string(),
        });
        
        let summarizer = AISummarizer::new(provider, "gpt-4");
        let messages = vec![
            Message::user("What's the project status?".to_string()),
            Message::assistant("We've completed three features this week.".to_string()),
        ];

        let result = summarizer.summarize(&messages).await.unwrap();
        
        assert!(result.is_summary());
        assert!(result.content.contains("Summary of 2 messages"));
        assert!(result.content.contains("project status"));
    }
}
