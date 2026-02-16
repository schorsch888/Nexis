//! Nexis protocol types and validation logic.
//!
//! This crate implements:
//! - NIP-001: member identity (`MemberId`)
//! - NIP-002: message envelope (`Message`)
//! - Permission actions and checks used by protocol-level authorization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberType {
    Human,
    Agent,
    Bot,
}

impl MemberType {
    pub fn as_str(&self) -> &str {
        match self {
            MemberType::Human => "human",
            MemberType::Agent => "agent",
            MemberType::Bot => "bot",
        }
    }
}

impl std::fmt::Display for MemberType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemberId(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemberIdError {
    #[error("invalid prefix: expected 'nexis:'")]
    InvalidPrefix,
    #[error("unknown member type: {0}")]
    InvalidType(String),
    #[error("invalid identifier: cannot be empty")]
    InvalidIdentifier,
}

impl MemberId {
    pub fn new(member_type: MemberType, identifier: &str) -> Result<Self, MemberIdError> {
        if identifier.is_empty() {
            return Err(MemberIdError::InvalidIdentifier);
        }
        Ok(Self(format!("nexis:{}:{}", member_type, identifier)))
    }

    pub fn member_type(&self) -> MemberType {
        let parts: Vec<&str> = self.0.split(':').collect();
        match parts.get(1).copied() {
            Some("human") => MemberType::Human,
            Some("agent") => MemberType::Agent,
            Some("bot") => MemberType::Bot,
            _ => MemberType::Human,
        }
    }

    pub fn identifier(&self) -> &str {
        let parts: Vec<&str> = self.0.split(':').collect();
        parts.get(2).unwrap_or(&"")
    }
}

impl std::str::FromStr for MemberId {
    type Err = MemberIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let prefix = "nexis:";
        if !s.starts_with(prefix) {
            return Err(MemberIdError::InvalidPrefix);
        }
        let rest = &s[prefix.len()..];
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() < 2 {
            return Err(MemberIdError::InvalidIdentifier);
        }
        let member_type = parts[0];
        let identifier = parts[1..].join(":");
        
        match member_type {
            "human" | "agent" | "bot" => {}
            _ => return Err(MemberIdError::InvalidType(member_type.to_string())),
        }
        
        if identifier.is_empty() {
            return Err(MemberIdError::InvalidIdentifier);
        }
        
        Ok(Self(s.to_string()))
    }
}

impl std::fmt::Display for MemberId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Read,
    Write,
    Invoke,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    #[serde(rename = "rooms")]
    pub allowed_rooms: Vec<String>,
    pub actions: Vec<Action>,
}

impl Permissions {
    pub fn new(allowed_rooms: Vec<String>, actions: Vec<Action>) -> Self {
        Self { allowed_rooms, actions }
    }

    pub fn can(&self, action: Action) -> bool {
        // Admin action implies all other actions
        if self.actions.contains(&Action::Admin) {
            return true;
        }
        self.actions.contains(&action)
    }

    pub fn can_access_room(&self, room_id: &str) -> bool {
        self.allowed_rooms.iter().any(|r| r == "*" || r == room_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageContent {
    Text { text: String },
    Code { code: String, language: Option<String> },
    Tool { tool_name: String, input: serde_json::Value },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    #[serde(rename = "roomId")]
    pub room_id: String,
    pub sender: MemberId,
    pub content: MessageContent,
    pub metadata: Option<serde_json::Value>,
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Message {
    pub fn new(
        id: String,
        room_id: String,
        sender: MemberId,
        content: MessageContent,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            room_id,
            sender,
            content,
            metadata: None,
            reply_to: None,
            created_at,
            updated_at: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("message id cannot be empty".to_string());
        }
        if self.room_id.is_empty() {
            return Err("room id cannot be empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::{Action, MemberId, MemberIdError, Message, MessageContent, Permissions};

    #[test]
    fn member_id_parses_valid_values() {
        let member = "nexis:human:alice@example.com".parse::<MemberId>().unwrap();
        assert_eq!(member.member_type().as_str(), "human");
        assert_eq!(member.identifier(), "alice@example.com");
        assert_eq!(member.to_string(), "nexis:human:alice@example.com");
    }

    #[test]
    fn member_id_rejects_invalid_prefix() {
        let err = "other:human:alice@example.com"
            .parse::<MemberId>()
            .unwrap_err();
        assert_eq!(err, MemberIdError::InvalidPrefix);
    }

    #[test]
    fn member_id_rejects_unknown_type() {
        let err = "nexis:robot:alice".parse::<MemberId>().unwrap_err();
        assert_eq!(err, MemberIdError::InvalidType("robot".to_string()));
    }

    #[test]
    fn member_id_rejects_empty_identifier() {
        let err = "nexis:agent:".parse::<MemberId>().unwrap_err();
        assert_eq!(err, MemberIdError::InvalidIdentifier);
    }

    #[test]
    fn member_id_json_round_trip_as_string() {
        let original = "nexis:agent:customer-support-v1"
            .parse::<MemberId>()
            .unwrap();
        let encoded = serde_json::to_string(&original).unwrap();
        assert_eq!(encoded, "\"nexis:agent:customer-support-v1\"");

        let decoded: MemberId = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn message_serializes_to_nip_002_shape() {
        let sender = "nexis:agent:openai/gpt-4".parse::<MemberId>().unwrap();
        let message = Message {
            id: "msg_abc123".to_string(),
            room_id: "room_xyz".to_string(),
            sender,
            content: MessageContent::Text {
                text: "hello".to_string(),
            },
            metadata: Some(json!({"model": "gpt-4"})),
            reply_to: Some("msg_def456".to_string()),
            created_at: Utc.with_ymd_and_hms(2026, 2, 14, 12, 0, 0).unwrap(),
            updated_at: None,
        };

        let encoded = serde_json::to_value(&message).unwrap();
        assert_eq!(encoded["roomId"], "room_xyz");
        assert_eq!(encoded["sender"], "nexis:agent:openai/gpt-4");
        assert_eq!(encoded["content"]["type"], "text");
        assert_eq!(encoded["content"]["text"], "hello");
        assert_eq!(encoded["replyTo"], "msg_def456");
        assert_eq!(encoded["metadata"]["model"], "gpt-4");
    }

    #[test]
    fn message_validation_rejects_blank_ids() {
        let sender = "nexis:human:alice@example.com".parse::<MemberId>().unwrap();
        let mut message = Message::new(
            "".to_string(),
            "room_xyz".to_string(),
            sender,
            MessageContent::Text {
                text: "hello".to_string(),
            },
            Utc::now(),
        );

        assert!(message.validate().is_err());

        message.id = "msg_1".to_string();
        message.room_id = "".to_string();
        assert!(message.validate().is_err());
    }

    #[test]
    fn permission_allows_wildcard_room_and_admin_action() {
        let permissions = Permissions::new(vec!["*".to_string()], vec![Action::Admin]);
        assert!(permissions.can_access_room("room_any"));
        assert!(permissions.can(Action::Read));
        assert!(permissions.can(Action::Write));
        assert!(permissions.can(Action::Invoke));
        assert!(permissions.can(Action::Admin));
    }

    #[test]
    fn permission_requires_exact_room_when_not_wildcard() {
        let permissions = Permissions::new(
            vec!["room_general".to_string()],
            vec![Action::Read, Action::Write],
        );

        assert!(permissions.can_access_room("room_general"));
        assert!(!permissions.can_access_room("room_private"));
        assert!(permissions.can(Action::Read));
        assert!(permissions.can(Action::Write));
        assert!(!permissions.can(Action::Invoke));
    }
}
