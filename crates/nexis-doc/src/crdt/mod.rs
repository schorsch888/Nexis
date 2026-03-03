//! CRDT primitives and synchronization stubs.

pub mod document;
pub mod sync;
pub mod types;

pub use document::{CRDTDocument, SubscriptionToken};
pub use sync::{DocumentSync, InMemorySyncProvider, SyncState};
pub use types::{CRDTOperation, ClientId, Clock, DocId};
