//! Nexis Context Management - Context windowing and management
//!
//! This crate provides:
//! - Context window management
//! - Token counting (optional, feature-gated)
//! - Conversation context tracking
//! - Context summarization (when window overflows)
//!
//! ## Features
//!
//! - `token-counting` - Enable accurate token counting using tokenizers
//! - `ai-summarizer` - Enable AI-powered summarization using nexis-runtime

pub mod context;
pub mod error;
pub mod manager;
pub mod summarizer;
pub mod window;

#[cfg(feature = "ai-summarizer")]
pub mod ai_summarizer;

#[cfg(feature = "ai-summarizer")]
pub use ai_summarizer::AISummarizer;

pub use context::{ConversationContext, Message, MessageRole};
pub use error::{ContextError, ContextResult};
pub use manager::ContextManager;
pub use summarizer::{ContextSummarizer, SummarizerConfig, NoOpSummarizer, MockSummarizer};
pub use window::{ContextWindow, OverflowStrategy};

/// Prelude for common imports
pub mod prelude {
    pub use crate::context::ConversationContext;
    pub use crate::error::{ContextError, ContextResult};
    pub use crate::manager::ContextManager;
    pub use crate::window::ContextWindow;
}
