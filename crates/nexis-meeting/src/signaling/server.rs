//! Minimal WebSocket-only signaling server.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{
    tungstenite::{Error as WsError, Message},
    WebSocketStream,
};

use super::types::{ParticipantId, RoomId};

#[derive(Debug, Default)]
struct ServerState {
    rooms: HashMap<RoomId, Vec<ParticipantId>>,
    peers: HashMap<ParticipantId, mpsc::UnboundedSender<Message>>,
}

/// Minimal signaling server with room membership state.
#[derive(Debug, Clone, Default)]
pub struct SignalingServer {
    state: Arc<RwLock<ServerState>>,
}

impl SignalingServer {
    /// Create a new signaling server.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle a single already-upgraded WebSocket connection.
    pub async fn handle_connection<S>(
        &self,
        ws_stream: WebSocketStream<S>,
        room_id: RoomId,
        participant_id: ParticipantId,
    ) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        {
            let mut state = self.state.write().await;
            state.peers.insert(participant_id, tx);
            state.rooms.entry(room_id).or_default().push(participant_id);
        }

        loop {
            tokio::select! {
                inbound = ws_receiver.next() => {
                    match inbound {
                        Some(Ok(message)) => {
                            if message.is_close() {
                                break;
                            }

                            self.broadcast_to_room(room_id, participant_id, message).await;
                        }
                        Some(Err(error)) => {
                            self.remove_participant(room_id, participant_id).await;
                            return Err(error);
                        }
                        None => {
                            break;
                        }
                    }
                }
                outbound = rx.recv() => {
                    match outbound {
                        Some(message) => ws_sender.send(message).await?,
                        None => break,
                    }
                }
            }
        }

        self.remove_participant(room_id, participant_id).await;
        Ok(())
    }

    /// Broadcast a message to all participants in a room except the sender.
    pub async fn broadcast_to_room(
        &self,
        room_id: RoomId,
        sender_id: ParticipantId,
        message: Message,
    ) {
        let recipients = {
            let state = self.state.read().await;
            state
                .rooms
                .get(&room_id)
                .into_iter()
                .flatten()
                .filter(|participant_id| **participant_id != sender_id)
                .filter_map(|participant_id| state.peers.get(participant_id).cloned())
                .collect::<Vec<_>>()
        };

        for recipient in recipients {
            let _ = recipient.send(message.clone());
        }
    }

    async fn remove_participant(&self, room_id: RoomId, participant_id: ParticipantId) {
        let mut state = self.state.write().await;

        state.peers.remove(&participant_id);

        if let Some(participants) = state.rooms.get_mut(&room_id) {
            participants.retain(|id| *id != participant_id);
            if participants.is_empty() {
                state.rooms.remove(&room_id);
            }
        }
    }
}
