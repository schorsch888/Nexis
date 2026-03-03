//! WebRTC signaling models and WebSocket server primitives.

pub mod protocol;
pub mod server;
pub mod types;

pub use protocol::{ErrorResponse, RoomEvent};
pub use server::SignalingServer;
pub use types::{ParticipantId, RoomId, SignalMessage, SignalType};
