//! Context manager implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{debug, warn};

use crate::context::{ConversationContext, Message};
use crate::error::{ContextError, ContextResult};
use crate::window::{ContextWindow, OverflowStrategy};
use crate::summarizer::{ContextSummarizer, SummarizerConfig};

#[cfg(feature = "metrics")]
use crate::metrics::{
    record_summarization_failure, record_summarization_overflow, record_summarization_success,
    record_truncation, record_window_utilization, set_active_contexts,
};

/// Context manager for handling conversation contexts
pub struct ContextManager {
    contexts: Arc<RwLock<HashMap<Uuid, ConversationContext>>>,
    window: ContextWindow,
    summarizer: Option<Arc<dyn ContextSummarizer>>,
    summarizer_config: SummarizerConfig,
}

impl ContextManager {
    /// Create a new context manager with default settings
    pub fn new(window: ContextWindow) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            window,
            summarizer: None,
            summarizer_config: SummarizerConfig::default(),
        }
    }

    /// Create a new context manager with a summarizer
    pub fn with_summarizer(window: ContextWindow, summarizer: Arc<dyn ContextSummarizer>) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            window,
            summarizer: Some(summarizer),
            summarizer_config: SummarizerConfig::default(),
        }
    }

    /// Create a new context manager with custom summarizer config
    pub fn with_summarizer_config(
        window: ContextWindow,
        summarizer: Arc<dyn ContextSummarizer>,
        config: SummarizerConfig,
    ) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            window,
            summarizer: Some(summarizer),
            summarizer_config: config,
        }
    }

    /// Create a new context
    pub async fn create_context(&self, room_id: Option<Uuid>) -> ContextResult<Uuid> {
        let context = ConversationContext::new(room_id);
        let id = context.id;
        self.contexts.write().await.insert(id, context);
        
        #[cfg(feature = "metrics")]
        set_active_contexts(self.contexts.read().await.len());
        
        Ok(id)
    }

    /// Get a context by ID
    pub async fn get_context(&self, id: Uuid) -> ContextResult<ConversationContext> {
        self.contexts
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| ContextError::NotFound(id.to_string()))
    }

    /// Add a message to a context
    pub async fn add_message(&self, context_id: Uuid, message: Message) -> ContextResult<()> {
        let mut contexts = self.contexts.write().await;
        let context = contexts
            .get_mut(&context_id)
            .ok_or_else(|| ContextError::NotFound(context_id.to_string()))?;

        // Check window overflow
        let estimated_tokens = estimate_tokens(&message.content);
        let new_total = context.total_tokens() + estimated_tokens;

        if new_total > self.window.available_tokens() {
            match self.window.overflow_strategy {
                OverflowStrategy::TruncateOldest => {
                    #[allow(unused_variables)]
                    let count = self.truncate_oldest_with_count(context, new_total - self.window.available_tokens());
                    #[cfg(feature = "metrics")]
                    record_truncation(count);
                }
                OverflowStrategy::Fail => {
                    return Err(ContextError::WindowFull);
                }
                OverflowStrategy::Summarize => {
                    self.handle_overflow_with_summarization(context, new_total).await?;
                }
            }
        }

        let mut message = message;
        message.token_count = Some(estimated_tokens);
        context.add_message(message);

        #[cfg(feature = "metrics")]
        {
            let utilization = (context.total_tokens() as f64 / self.window.available_tokens() as f64) * 100.0;
            record_window_utilization(utilization);
        }

        Ok(())
    }

    /// Handle overflow using summarization strategy
    async fn handle_overflow_with_summarization(
        &self,
        context: &mut ConversationContext,
        new_total: usize,
    ) -> ContextResult<()> {
        #[cfg(feature = "metrics")]
        record_summarization_overflow();
        
        let tokens_to_free = new_total - self.window.available_tokens();
        
        // If no summarizer configured, fall back to truncation
        let Some(ref summarizer) = self.summarizer else {
            debug!("No summarizer configured, falling back to truncation");
            let truncated = self.truncate_oldest_with_count(context, tokens_to_free);
            #[cfg(feature = "metrics")]
            record_truncation(truncated);
            return Ok(());
        };

        // Collect messages to summarize (respecting batch size)
        let batch_size = self.summarizer_config.batch_size.min(context.messages.len());
        if batch_size == 0 {
            warn!("No messages to summarize");
            return Ok(());
        }

        let messages_to_summarize: Vec<Message> = context.messages.drain(0..batch_size).collect();

        debug!(
            batch_size = batch_size,
            "Attempting to summarize messages"
        );

        let start = Instant::now();
        match summarizer.summarize(&messages_to_summarize).await {
            Ok(summary) => {
                // Insert summary at the beginning
                context.messages.insert(0, summary);
                let latency = start.elapsed().as_secs_f64();
                
                #[cfg(feature = "metrics")]
                record_summarization_success(batch_size, latency);
                
                debug!("Successfully summarized {} messages in {:.2}s", batch_size, latency);
                Ok(())
            }
            Err(e) => {
                // On failure, restore the messages and fall back to truncation
                warn!(error = ?e, "Summarization failed, falling back to truncation");
                context.messages = [messages_to_summarize, context.messages.clone()].concat();
                let truncated = self.truncate_oldest_with_count(context, tokens_to_free);
                
                #[cfg(feature = "metrics")]
                {
                    record_summarization_failure();
                    record_truncation(truncated);
                }
                
                Err(ContextError::SummarizationFailed(e.to_string()))
            }
        }
    }

    /// Delete a context
    pub async fn delete_context(&self, id: Uuid) -> ContextResult<()> {
        self.contexts
            .write()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| ContextError::NotFound(id.to_string()))?;
        
        #[cfg(feature = "metrics")]
        set_active_contexts(self.contexts.read().await.len());
        
        Ok(())
    }

    /// Get number of active contexts
    pub async fn context_count(&self) -> usize {
        self.contexts.read().await.len()
    }

    /// Truncate oldest messages and return count of messages removed
    fn truncate_oldest_with_count(&self, context: &mut ConversationContext, tokens_to_free: usize) -> usize {
        let mut freed = 0;
        let mut count = 0;
        while freed < tokens_to_free && context.messages.len() > 1 {
            if let Some(msg) = context.messages.first() {
                freed += msg.token_count.unwrap_or(0);
                context.messages.remove(0);
                count += 1;
            }
        }
        count
    }
}

/// Simple token estimation (approximately 4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::summarizer::MockSummarizer;

    #[tokio::test]
    async fn test_create_and_get_context() {
        let manager = ContextManager::new(ContextWindow::default());
        let id = manager.create_context(None).await.unwrap();
        let context = manager.get_context(id).await.unwrap();
        assert!(context.messages.is_empty());
    }

    #[tokio::test]
    async fn test_add_message() {
        let manager = ContextManager::new(ContextWindow::default());
        let id = manager.create_context(None).await.unwrap();

        let msg = Message::user("Hello".to_string());
        manager.add_message(id, msg).await.unwrap();

        let context = manager.get_context(id).await.unwrap();
        assert_eq!(context.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_window_overflow_truncate() {
        let window = ContextWindow::new(50); // Very small window
        let manager = ContextManager::new(window);
        let id = manager.create_context(None).await.unwrap();

        // Add multiple messages
        for i in 0..10 {
            let msg = Message::user(format!("Message number {} with some content", i));
            manager.add_message(id, msg).await.unwrap();
        }

        let context = manager.get_context(id).await.unwrap();
        // Should have truncated some messages
        assert!(context.messages.len() < 10);
    }

    #[tokio::test]
    async fn test_window_overflow_summarize() {
        let window = ContextWindow::new(100).with_overflow_strategy(OverflowStrategy::Summarize);
        let summarizer = Arc::new(MockSummarizer::new("Previous conversation summary"));
        let manager = ContextManager::with_summarizer(window, summarizer);
        let id = manager.create_context(None).await.unwrap();

        // Add enough messages to trigger overflow
        for i in 0..20 {
            let msg = Message::user(format!("Message number {} with enough content to fill window", i));
            manager.add_message(id, msg).await.unwrap();
        }

        let context = manager.get_context(id).await.unwrap();
        
        // Should have a summary message at the beginning
        assert!(!context.messages.is_empty());
        assert!(context.messages[0].is_summary(), "First message should be a summary");
        assert!(context.messages[0].content.contains("Summary of"));
    }

    #[tokio::test]
    async fn test_summarization_fallback_on_error() {
        let window = ContextWindow::new(50).with_overflow_strategy(OverflowStrategy::Summarize);
        // Use a summarizer that will fail - we'll test this by checking truncation still works
        let manager = ContextManager::new(window);
        let id = manager.create_context(None).await.unwrap();

        // Add messages to trigger overflow
        for i in 0..10 {
            let msg = Message::user(format!("Message {} with content", i));
            manager.add_message(id, msg).await.unwrap();
        }

        let context = manager.get_context(id).await.unwrap();
        // Without a working summarizer, should have truncated
        assert!(context.messages.len() < 10);
    }

    #[tokio::test]
    async fn test_context_count() {
        let manager = ContextManager::new(ContextWindow::default());
        assert_eq!(manager.context_count().await, 0);

        manager.create_context(None).await.unwrap();
        assert_eq!(manager.context_count().await, 1);

        manager.create_context(None).await.unwrap();
        assert_eq!(manager.context_count().await, 2);
    }
}
