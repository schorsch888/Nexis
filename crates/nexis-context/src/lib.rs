//! Nexis Context Management - Context windowing and management
//!
//! This crate provides:
//! - Context window management
//! - Token counting (optional, feature-gated)
//! - Conversation context tracking

pub mod context;
pub mod error;
pub mod manager;
pub mod window;

pub use context::ConversationContext;
pub use error::{ContextError, ContextResult};
pub use manager::ContextManager;
pub use window::ContextWindow;

/// Prelude for common imports
pub mod prelude {
    pub use crate::context::ConversationContext;
    pub use crate::error::{ContextError, ContextResult};
    pub use crate::manager::ContextManager;
    pub use crate::window::ContextWindow;
}
