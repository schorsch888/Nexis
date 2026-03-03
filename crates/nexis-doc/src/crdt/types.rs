//! Shared CRDT types.

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use uuid::Uuid;

/// Stable document identifier.
pub type DocId = Uuid;

/// Logical client identifier used by CRDT replicas.
pub type ClientId = u64;

/// Compact vector-clock representation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    pub entries: SmallVec<[(ClientId, u64); 4]>,
}

/// High-level CRDT text operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CRDTOperation {
    Insert { index: u32, content: String },
    Delete { index: u32, len: u32 },
    Retain { len: u32 },
}
