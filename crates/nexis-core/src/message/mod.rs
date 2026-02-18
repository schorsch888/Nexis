//! Message domain extensions for Nexis.

pub use nexis_protocol::{Message, MessageContent};

#[derive(Debug, Clone)]
pub struct MessageBuilder {
    id: String,
    room_id: String,
    sender: nexis_protocol::MemberId,
    content: MessageContent,
    reply_to: Option<String>,
}

impl MessageBuilder {
    pub fn new(
        id: String,
        room_id: String,
        sender: nexis_protocol::MemberId,
        content: MessageContent,
    ) -> Self {
        Self {
            id,
            room_id,
            sender,
            content,
            reply_to: None,
        }
    }

    pub fn with_reply_to(mut self, reply_to: String) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    pub fn build(self) -> Message {
        Message {
            id: self.id,
            room_id: self.room_id,
            sender: self.sender,
            content: self.content,
            metadata: None,
            reply_to: self.reply_to,
            created_at: chrono::Utc::now(),
            updated_at: None,
        }
    }
}
