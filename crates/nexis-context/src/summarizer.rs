//! Context summarization support
//!
//! Provides trait and implementations for summarizing conversation context
//! when the window overflows.

use async_trait::async_trait;
use crate::context::{Message, MessageRole};
use crate::error::{ContextError, ContextResult};

/// Trait for context summarization strategies
#[async_trait]
pub trait ContextSummarizer: Send + Sync + std::fmt::Debug {
    /// Summarize a list of messages into a single summary message
    ///
    /// # Arguments
    /// * `messages` - Messages to summarize (typically the oldest N messages)
    ///
    /// # Returns
    /// A summary message that captures the key information from the input messages
    async fn summarize(&self, messages: &[Message]) -> ContextResult<Message>;
}

/// Configuration for summarization behavior
#[derive(Debug, Clone)]
pub struct SummarizerConfig {
    /// Maximum tokens for the summary output
    pub max_summary_tokens: usize,
    /// Number of messages to summarize at once
    pub batch_size: usize,
    /// System prompt template for summarization
    pub prompt_template: String,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            max_summary_tokens: 500,
            batch_size: 10,
            prompt_template: "Summarize the following conversation, preserving key information, decisions, and context. Be concise but comprehensive:\n\n{messages}".to_string(),
        }
    }
}

impl SummarizerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.max_summary_tokens = tokens;
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Format messages into a string for summarization
    pub fn format_messages(&self, messages: &[Message]) -> String {
        let formatted: Vec<String> = messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                };
                format!("{}: {}", role, m.content)
            })
            .collect();

        self.prompt_template.replace("{messages}", &formatted.join("\n"))
    }
}

/// No-op summarizer that always fails (fallback for when summarization is disabled)
#[derive(Debug, Clone)]
pub struct NoOpSummarizer;

#[async_trait]
impl ContextSummarizer for NoOpSummarizer {
    async fn summarize(&self, _messages: &[Message]) -> ContextResult<Message> {
        Err(ContextError::SummarizationNotAvailable)
    }
}

/// Mock summarizer for testing
#[derive(Debug, Clone)]
pub struct MockSummarizer {
    /// Summary to return
    pub summary: String,
}

impl MockSummarizer {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
        }
    }
}

#[async_trait]
impl ContextSummarizer for MockSummarizer {
    async fn summarize(&self, messages: &[Message]) -> ContextResult<Message> {
        // Count messages for the summary
        let count = messages.len();
        let mut msg = Message::system(format!(
            "[Summary of {} messages] {}",
            count, self.summary
        ));
        msg.token_count = Some(count * 10); // Rough estimate
        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_format_messages() {
        let config = SummarizerConfig::new();
        let messages = vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi there!".to_string()),
        ];

        let formatted = config.format_messages(&messages);

        assert!(formatted.contains("User: Hello"));
        assert!(formatted.contains("Assistant: Hi there!"));
    }

    #[tokio::test]
    async fn test_noop_summarizer_fails() {
        let summarizer = NoOpSummarizer;
        let messages = vec![Message::user("test".to_string())];

        let result = summarizer.summarize(&messages).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_summarizer_returns_summary() {
        let summarizer = MockSummarizer::new("Test summary");
        let messages = vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi!".to_string()),
        ];

        let result = summarizer.summarize(&messages).await.unwrap();

        assert_eq!(result.role, MessageRole::System);
        assert!(result.content.contains("Summary of 2 messages"));
        assert!(result.content.contains("Test summary"));
    }
}
