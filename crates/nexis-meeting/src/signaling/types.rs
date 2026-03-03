//! Core WebRTC signaling message models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Identifier for a meeting room.
pub type RoomId = Uuid;

/// Identifier for a room participant.
pub type ParticipantId = Uuid;

/// Signaling message kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    Offer,
    Answer,
    IceCandidate,
    Join,
    Leave,
    ParticipantUpdate,
    Error,
}

/// Generic signaling payload envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalMessage {
    pub id: Uuid,
    pub room_id: RoomId,
    pub from_participant_id: ParticipantId,
    pub to_participant_id: Option<ParticipantId>,
    pub signal_type: SignalType,
    pub payload: Value,
    pub sent_at: DateTime<Utc>,
}
