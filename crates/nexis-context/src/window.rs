//! Context window management

use serde::{Deserialize, Serialize};

/// Context window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Maximum tokens in the context window
    pub max_tokens: usize,
    /// Reserved tokens for system messages
    pub reserved_tokens: usize,
    /// Strategy for window overflow
    pub overflow_strategy: OverflowStrategy,
}

impl Default for ContextWindow {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            reserved_tokens: 256,
            overflow_strategy: OverflowStrategy::TruncateOldest,
        }
    }
}

impl ContextWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            ..Default::default()
        }
    }

    pub fn available_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserved_tokens)
    }
}

/// Strategy for handling context overflow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OverflowStrategy {
    /// Remove oldest messages
    TruncateOldest,
    /// Summarize old messages
    Summarize,
    /// Fail with error
    Fail,
}
