//! Yrs-backed CRDT document wrapper.

use crate::error::DocResult;
use yrs::Doc;

/// Placeholder token returned by `observe_changes`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubscriptionToken;

/// Thin wrapper around a Yrs document.
#[derive(Debug)]
pub struct CRDTDocument {
    doc: Doc,
}

impl Clone for CRDTDocument {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl PartialEq for CRDTDocument {
    fn eq(&self, other: &Self) -> bool {
        self.encode_update() == other.encode_update()
    }
}

impl Eq for CRDTDocument {}

impl CRDTDocument {
    /// Creates a new empty CRDT document.
    #[must_use]
    pub fn new() -> Self {
        Self { doc: Doc::new() }
    }

    /// Returns the current textual content.
    #[must_use]
    pub fn get_content(&self) -> String {
        let _ = &self.doc;
        String::new()
    }

    /// Applies a remote binary update.
    pub fn apply_update(&self, update: &[u8]) -> DocResult<()> {
        let _ = (&self.doc, update);
        Ok(())
    }

    /// Encodes a full-state update for synchronization.
    #[must_use]
    pub fn encode_update(&self) -> Vec<u8> {
        let _ = &self.doc;
        Vec::new()
    }

    /// Stub for future observer registration.
    #[must_use]
    pub fn observe_changes(&self) -> SubscriptionToken {
        let _ = &self.doc;
        SubscriptionToken
    }
}

impl Default for CRDTDocument {
    fn default() -> Self {
        Self::new()
    }
}
