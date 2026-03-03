//! Synchronization interfaces and in-memory test provider.

use crate::crdt::types::{Clock, DocId};
use std::collections::HashMap;

/// Per-document synchronization metadata.
#[derive(Debug, Clone, Default)]
pub struct SyncState {
    pub clock: Clock,
    pub last_update_len: usize,
}

/// Basic sync abstraction for pushing/pulling document updates.
pub trait DocumentSync {
    fn push_update(&mut self, doc_id: DocId, update: Vec<u8>);
    fn pull_update(&self, doc_id: DocId) -> Option<Vec<u8>>;
    fn state(&self, doc_id: DocId) -> Option<&SyncState>;
}

/// In-memory sync provider intended for tests.
#[derive(Debug, Default)]
pub struct InMemorySyncProvider {
    updates: HashMap<DocId, Vec<u8>>,
    states: HashMap<DocId, SyncState>,
}

impl InMemorySyncProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl DocumentSync for InMemorySyncProvider {
    fn push_update(&mut self, doc_id: DocId, update: Vec<u8>) {
        let len = update.len();
        self.updates.insert(doc_id, update);

        let state = self.states.entry(doc_id).or_default();
        state.last_update_len = len;
    }

    fn pull_update(&self, doc_id: DocId) -> Option<Vec<u8>> {
        self.updates.get(&doc_id).cloned()
    }

    fn state(&self, doc_id: DocId) -> Option<&SyncState> {
        self.states.get(&doc_id)
    }
}
