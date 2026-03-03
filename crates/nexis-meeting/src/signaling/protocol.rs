//! Signaling protocol events and error payloads.

use serde::{Deserialize, Serialize};

use super::types::{ParticipantId, RoomId};

/// Room-scoped participant lifecycle events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum RoomEvent {
    Joined {
        room_id: RoomId,
        participant_id: ParticipantId,
    },
    Left {
        room_id: RoomId,
        participant_id: ParticipantId,
    },
    ParticipantUpdated {
        room_id: RoomId,
        participant_id: ParticipantId,
    },
}

/// Protocol-level error envelope sent over signaling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub room_id: Option<RoomId>,
    pub participant_id: Option<ParticipantId>,
}
