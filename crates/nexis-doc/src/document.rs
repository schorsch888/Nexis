//! Document metadata and version models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crdt::CRDTDocument;

/// Document version metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocVersion {
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub checksum: Option<String>,
}

/// Document metadata fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocMetadata {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Collaboratively edited document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub metadata: DocMetadata,
    pub content: String,
    pub current_version: DocVersion,
    #[serde(skip, default)]
    pub crdt_doc: Option<CRDTDocument>,
}
