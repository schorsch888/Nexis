//! Context manager implementation

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::context::{ConversationContext, Message};
use crate::error::{ContextError, ContextResult};
use crate::window::{ContextWindow, OverflowStrategy};

/// Context manager for handling conversation contexts
pub struct ContextManager {
    contexts: Arc<RwLock<HashMap<Uuid, ConversationContext>>>,
    window: ContextWindow,
}

impl ContextManager {
    pub fn new(window: ContextWindow) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            window,
        }
    }

    /// Create a new context
    pub async fn create_context(&self, room_id: Option<Uuid>) -> ContextResult<Uuid> {
        let context = ConversationContext::new(room_id);
        let id = context.id;
        self.contexts.write().await.insert(id, context);
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
                    truncate_oldest(context, new_total - self.window.available_tokens());
                }
                OverflowStrategy::Fail => {
                    return Err(ContextError::WindowFull);
                }
                OverflowStrategy::Summarize => {
                    // TODO: Implement summarization
                    truncate_oldest(context, new_total - self.window.available_tokens());
                }
            }
        }

        let mut message = message;
        message.token_count = Some(estimated_tokens);
        context.add_message(message);

        Ok(())
    }

    /// Delete a context
    pub async fn delete_context(&self, id: Uuid) -> ContextResult<()> {
        self.contexts
            .write()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| ContextError::NotFound(id.to_string()))
    }
}

/// Simple token estimation (approximately 4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Truncate oldest messages to free up tokens
fn truncate_oldest(context: &mut ConversationContext, tokens_to_free: usize) {
    let mut freed = 0;
    while freed < tokens_to_free && context.messages.len() > 1 {
        if let Some(msg) = context.messages.first() {
            freed += msg.token_count.unwrap_or(0);
            context.messages.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    async fn test_window_overflow() {
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
}
