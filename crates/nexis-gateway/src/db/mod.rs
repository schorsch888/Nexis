//! Database and repository layer for message persistence.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
#[cfg(feature = "persistence-sqlx")]
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use thiserror::Error;

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use tokio::sync::RwLock;

/// Database connection pool type used by gateway persistence.
#[cfg(feature = "persistence-sqlx")]
pub type DatabasePool = PgPool;

/// Placeholder pool type when SQLx persistence is disabled.
#[cfg(not(feature = "persistence-sqlx"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct DatabasePool;

/// SQL schema for the `rooms` table.
pub const ROOMS_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS rooms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    topic TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);"#;

/// SQL schema for the `messages` table.
pub const MESSAGES_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    sender_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);"#;

/// SQL schema for the `members` table.
pub const MEMBERS_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS members (
    id TEXT PRIMARY KEY,
    "type" TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);"#;

/// Error type returned by repository operations.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// Database query failed.
    #[cfg(feature = "persistence-sqlx")]
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    /// SQLx persistence feature is disabled.
    #[error("persistence-sqlx feature is disabled")]
    SqlxDisabled,
}

/// Domain model for a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Room {
    /// Room ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Optional topic.
    pub topic: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Tenant ID (multi-tenant only).
    #[cfg(feature = "multi-tenant")]
    pub tenant_id: Option<String>,
}

/// Domain model for a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Message ID.
    pub id: String,
    /// Parent room ID.
    pub room_id: String,
    /// Sender member ID.
    pub sender_id: String,
    /// Message body.
    pub content: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Tenant ID (multi-tenant only).
    #[cfg(feature = "multi-tenant")]
    pub tenant_id: Option<String>,
}

/// Domain model for a member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Member ID.
    pub id: String,
    /// Member type (`human`, `ai`, `agent`, ...).
    pub member_type: String,
    /// Unique email.
    pub email: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Tenant ID (multi-tenant only).
    #[cfg(feature = "multi-tenant")]
    pub tenant_id: Option<String>,
}

/// Create a PostgreSQL connection pool for gateway persistence.
#[cfg(feature = "persistence-sqlx")]
pub async fn init_pool(database_url: &str) -> Result<DatabasePool, RepositoryError> {
    Ok(PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?)
}

/// Create a PostgreSQL connection pool for gateway persistence.
#[cfg(not(feature = "persistence-sqlx"))]
pub async fn init_pool(_database_url: &str) -> Result<DatabasePool, RepositoryError> {
    Err(RepositoryError::SqlxDisabled)
}

/// Initialize required tables if they do not exist.
#[cfg(feature = "persistence-sqlx")]
pub async fn initialize_schema(pool: &DatabasePool) -> Result<(), RepositoryError> {
    sqlx::query(ROOMS_TABLE_SCHEMA).execute(pool).await?;
    sqlx::query(MESSAGES_TABLE_SCHEMA).execute(pool).await?;
    sqlx::query(MEMBERS_TABLE_SCHEMA).execute(pool).await?;
    Ok(())
}

/// Initialize required tables if they do not exist.
#[cfg(not(feature = "persistence-sqlx"))]
pub async fn initialize_schema(_pool: &DatabasePool) -> Result<(), RepositoryError> {
    Err(RepositoryError::SqlxDisabled)
}

/// Persistence operations for rooms.
#[async_trait]
pub trait RoomRepository: Send + Sync {
    /// Create and persist a room.
    async fn create(&self, name: &str, topic: Option<&str>) -> Result<Room, RepositoryError>;
    /// Load one room by ID.
    async fn get(&self, id: &str) -> Result<Option<Room>, RepositoryError>;
    /// List all rooms.
    async fn list(&self) -> Result<Vec<Room>, RepositoryError>;

    /// Create room with tenant context (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        name: &str,
        topic: Option<&str>,
    ) -> Result<Room, RepositoryError>;
    /// Get room by ID within tenant scope (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(&self, tenant_id: &str, id: &str) -> Result<Option<Room>, RepositoryError>;
    /// List rooms within tenant scope (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn list_tenant(&self, tenant_id: &str) -> Result<Vec<Room>, RepositoryError>;
}

/// Persistence operations for messages.
#[async_trait]
pub trait MessageRepository: Send + Sync {
    /// Create and persist a message.
    async fn create(
        &self,
        room_id: &str,
        sender_id: &str,
        content: &str,
    ) -> Result<Message, RepositoryError>;
    /// Load one message by ID.
    async fn get(&self, id: &str) -> Result<Option<Message>, RepositoryError>;
    /// List all messages in a room.
    async fn list_by_room(&self, room_id: &str) -> Result<Vec<Message>, RepositoryError>;

    /// Create message with tenant context (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        room_id: &str,
        sender_id: &str,
        content: &str,
    ) -> Result<Message, RepositoryError>;
    /// Get message by ID within tenant scope (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Message>, RepositoryError>;
    /// List messages in room within tenant scope (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn list_by_room_tenant(
        &self,
        tenant_id: &str,
        room_id: &str,
    ) -> Result<Vec<Message>, RepositoryError>;
}

/// Persistence operations for members.
#[async_trait]
pub trait MemberRepository: Send + Sync {
    /// Create and persist a member.
    async fn create(&self, member_type: &str, email: &str) -> Result<Member, RepositoryError>;
    /// Load one member by ID.
    async fn get(&self, id: &str) -> Result<Option<Member>, RepositoryError>;

    /// Create member with tenant context (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        member_type: &str,
        email: &str,
    ) -> Result<Member, RepositoryError>;
    /// Get member by ID within tenant scope (multi-tenant).
    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Member>, RepositoryError>;
}

/// SQLx/PostgreSQL implementation of [`RoomRepository`].
#[cfg(feature = "persistence-sqlx")]
#[derive(Debug, Clone)]
pub struct SqlxRoomRepository {
    pool: DatabasePool,
}

#[cfg(feature = "persistence-sqlx")]
impl SqlxRoomRepository {
    /// Build a repository over an existing pool.
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "persistence-sqlx")]
#[async_trait]
impl RoomRepository for SqlxRoomRepository {
    async fn create(&self, name: &str, topic: Option<&str>) -> Result<Room, RepositoryError> {
        let id = format!("room_{}", Uuid::new_v4().simple());
        let row = sqlx::query(
            "INSERT INTO rooms (id, name, topic) VALUES ($1, $2, $3) RETURNING id, name, topic, created_at",
        )
        .bind(&id)
        .bind(name)
        .bind(topic)
        .fetch_one(&self.pool)
        .await?;

        Ok(Room {
            id: row.get("id"),
            name: row.get("name"),
            topic: row.get("topic"),
            created_at: row.get("created_at"),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        })
    }

    async fn get(&self, id: &str) -> Result<Option<Room>, RepositoryError> {
        let row = sqlx::query("SELECT id, name, topic, created_at FROM rooms WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|row| Room {
            id: row.get("id"),
            name: row.get("name"),
            topic: row.get("topic"),
            created_at: row.get("created_at"),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        }))
    }

    async fn list(&self) -> Result<Vec<Room>, RepositoryError> {
        let rows =
            sqlx::query("SELECT id, name, topic, created_at FROM rooms ORDER BY created_at ASC")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|row| Room {
                id: row.get("id"),
                name: row.get("name"),
                topic: row.get("topic"),
                created_at: row.get("created_at"),
                #[cfg(feature = "multi-tenant")]
                tenant_id: None,
            })
            .collect())
    }

    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        name: &str,
        topic: Option<&str>,
    ) -> Result<Room, RepositoryError> {
        let id = format!("room_{}", Uuid::new_v4().simple());
        let row = sqlx::query(
            "INSERT INTO rooms (id, name, topic, tenant_id) VALUES ($1, $2, $3, $4) RETURNING id, name, topic, created_at, tenant_id",
        )
        .bind(&id)
        .bind(name)
        .bind(topic)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Room {
            id: row.get("id"),
            name: row.get("name"),
            topic: row.get("topic"),
            created_at: row.get("created_at"),
            tenant_id: row.get("tenant_id"),
        })
    }

    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(&self, tenant_id: &str, id: &str) -> Result<Option<Room>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, name, topic, created_at, tenant_id FROM rooms WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Room {
            id: row.get("id"),
            name: row.get("name"),
            topic: row.get("topic"),
            created_at: row.get("created_at"),
            tenant_id: row.get("tenant_id"),
        }))
    }

    #[cfg(feature = "multi-tenant")]
    async fn list_tenant(&self, tenant_id: &str) -> Result<Vec<Room>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, name, topic, created_at, tenant_id FROM rooms WHERE tenant_id = $1 ORDER BY created_at ASC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Room {
                id: row.get("id"),
                name: row.get("name"),
                topic: row.get("topic"),
                created_at: row.get("created_at"),
                tenant_id: row.get("tenant_id"),
            })
            .collect())
    }
}

/// SQLx/PostgreSQL implementation of [`MessageRepository`].
#[cfg(feature = "persistence-sqlx")]
#[derive(Debug, Clone)]
pub struct SqlxMessageRepository {
    pool: DatabasePool,
}

#[cfg(feature = "persistence-sqlx")]
impl SqlxMessageRepository {
    /// Build a repository over an existing pool.
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "persistence-sqlx")]
#[async_trait]
impl MessageRepository for SqlxMessageRepository {
    async fn create(
        &self,
        room_id: &str,
        sender_id: &str,
        content: &str,
    ) -> Result<Message, RepositoryError> {
        let id = format!("msg_{}", Uuid::new_v4().simple());
        let row = sqlx::query(
            "INSERT INTO messages (id, room_id, sender_id, content) VALUES ($1, $2, $3, $4) RETURNING id, room_id, sender_id, content, created_at",
        )
        .bind(&id)
        .bind(room_id)
        .bind(sender_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await?;

        Ok(Message {
            id: row.get("id"),
            room_id: row.get("room_id"),
            sender_id: row.get("sender_id"),
            content: row.get("content"),
            created_at: row.get("created_at"),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        })
    }

    async fn get(&self, id: &str) -> Result<Option<Message>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, room_id, sender_id, content, created_at FROM messages WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Message {
            id: row.get("id"),
            room_id: row.get("room_id"),
            sender_id: row.get("sender_id"),
            content: row.get("content"),
            created_at: row.get("created_at"),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        }))
    }

    async fn list_by_room(&self, room_id: &str) -> Result<Vec<Message>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, room_id, sender_id, content, created_at FROM messages WHERE room_id = $1 ORDER BY created_at ASC",
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Message {
                id: row.get("id"),
                room_id: row.get("room_id"),
                sender_id: row.get("sender_id"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                #[cfg(feature = "multi-tenant")]
                tenant_id: None,
            })
            .collect())
    }

    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        room_id: &str,
        sender_id: &str,
        content: &str,
    ) -> Result<Message, RepositoryError> {
        let id = format!("msg_{}", Uuid::new_v4().simple());
        let row = sqlx::query(
            "INSERT INTO messages (id, room_id, sender_id, content, tenant_id) VALUES ($1, $2, $3, $4, $5) RETURNING id, room_id, sender_id, content, created_at, tenant_id",
        )
        .bind(&id)
        .bind(room_id)
        .bind(sender_id)
        .bind(content)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Message {
            id: row.get("id"),
            room_id: row.get("room_id"),
            sender_id: row.get("sender_id"),
            content: row.get("content"),
            created_at: row.get("created_at"),
            tenant_id: row.get("tenant_id"),
        })
    }

    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Message>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, room_id, sender_id, content, created_at, tenant_id FROM messages WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Message {
            id: row.get("id"),
            room_id: row.get("room_id"),
            sender_id: row.get("sender_id"),
            content: row.get("content"),
            created_at: row.get("created_at"),
            tenant_id: row.get("tenant_id"),
        }))
    }

    #[cfg(feature = "multi-tenant")]
    async fn list_by_room_tenant(
        &self,
        tenant_id: &str,
        room_id: &str,
    ) -> Result<Vec<Message>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, room_id, sender_id, content, created_at, tenant_id FROM messages WHERE room_id = $1 AND tenant_id = $2 ORDER BY created_at ASC",
        )
        .bind(room_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Message {
                id: row.get("id"),
                room_id: row.get("room_id"),
                sender_id: row.get("sender_id"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                tenant_id: row.get("tenant_id"),
            })
            .collect())
    }
}

/// SQLx/PostgreSQL implementation of [`MemberRepository`].
#[cfg(feature = "persistence-sqlx")]
#[derive(Debug, Clone)]
pub struct SqlxMemberRepository {
    pool: DatabasePool,
}

#[cfg(feature = "persistence-sqlx")]
impl SqlxMemberRepository {
    /// Build a repository over an existing pool.
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "persistence-sqlx")]
#[async_trait]
impl MemberRepository for SqlxMemberRepository {
    async fn create(&self, member_type: &str, email: &str) -> Result<Member, RepositoryError> {
        let id = format!("member_{}", Uuid::new_v4().simple());
        let row = sqlx::query(
            r#"INSERT INTO members (id, "type", email) VALUES ($1, $2, $3) RETURNING id, "type", email, created_at"#,
        )
        .bind(&id)
        .bind(member_type)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(Member {
            id: row.get("id"),
            member_type: row.get("type"),
            email: row.get("email"),
            created_at: row.get("created_at"),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        })
    }

    async fn get(&self, id: &str) -> Result<Option<Member>, RepositoryError> {
        let row = sqlx::query(r#"SELECT id, "type", email, created_at FROM members WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|row| Member {
            id: row.get("id"),
            member_type: row.get("type"),
            email: row.get("email"),
            created_at: row.get("created_at"),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        }))
    }

    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        member_type: &str,
        email: &str,
    ) -> Result<Member, RepositoryError> {
        let id = format!("member_{}", Uuid::new_v4().simple());
        let row = sqlx::query(
            r#"INSERT INTO members (id, "type", email, tenant_id) VALUES ($1, $2, $3, $4) RETURNING id, "type", email, created_at, tenant_id"#,
        )
        .bind(&id)
        .bind(member_type)
        .bind(email)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Member {
            id: row.get("id"),
            member_type: row.get("type"),
            email: row.get("email"),
            created_at: row.get("created_at"),
            tenant_id: row.get("tenant_id"),
        })
    }

    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Member>, RepositoryError> {
        let row = sqlx::query(
            r#"SELECT id, "type", email, created_at, tenant_id FROM members WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Member {
            id: row.get("id"),
            member_type: row.get("type"),
            email: row.get("email"),
            created_at: row.get("created_at"),
            tenant_id: row.get("tenant_id"),
        }))
    }
}

#[cfg(test)]
#[derive(Debug, Default, Clone)]
struct InMemoryRoomRepository {
    rooms: Arc<RwLock<HashMap<String, Room>>>,
}

#[cfg(test)]
#[async_trait]
impl RoomRepository for InMemoryRoomRepository {
    async fn create(&self, name: &str, topic: Option<&str>) -> Result<Room, RepositoryError> {
        let room = Room {
            id: format!("room_{}", Uuid::new_v4().simple()),
            name: name.to_string(),
            topic: topic.map(std::string::ToString::to_string),
            created_at: Utc::now(),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        };

        self.rooms
            .write()
            .await
            .insert(room.id.clone(), room.clone());
        Ok(room)
    }

    async fn get(&self, id: &str) -> Result<Option<Room>, RepositoryError> {
        Ok(self.rooms.read().await.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<Room>, RepositoryError> {
        let mut rooms = self
            .rooms
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        rooms.sort_by_key(|room| room.created_at);
        Ok(rooms)
    }

    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        name: &str,
        topic: Option<&str>,
    ) -> Result<Room, RepositoryError> {
        let room = Room {
            id: format!("room_{}", Uuid::new_v4().simple()),
            name: name.to_string(),
            topic: topic.map(std::string::ToString::to_string),
            created_at: Utc::now(),
            tenant_id: Some(tenant_id.to_string()),
        };

        self.rooms
            .write()
            .await
            .insert(room.id.clone(), room.clone());
        Ok(room)
    }

    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(&self, tenant_id: &str, id: &str) -> Result<Option<Room>, RepositoryError> {
        Ok(self
            .rooms
            .read()
            .await
            .get(id)
            .filter(|room| room.tenant_id.as_deref() == Some(tenant_id))
            .cloned())
    }

    #[cfg(feature = "multi-tenant")]
    async fn list_tenant(&self, tenant_id: &str) -> Result<Vec<Room>, RepositoryError> {
        let mut rooms = self
            .rooms
            .read()
            .await
            .values()
            .filter(|room| room.tenant_id.as_deref() == Some(tenant_id))
            .cloned()
            .collect::<Vec<_>>();
        rooms.sort_by_key(|room| room.created_at);
        Ok(rooms)
    }
}

#[cfg(test)]
#[derive(Debug, Default, Clone)]
struct InMemoryMessageRepository {
    messages: Arc<RwLock<HashMap<String, Message>>>,
}

#[cfg(test)]
#[async_trait]
impl MessageRepository for InMemoryMessageRepository {
    async fn create(
        &self,
        room_id: &str,
        sender_id: &str,
        content: &str,
    ) -> Result<Message, RepositoryError> {
        let message = Message {
            id: format!("msg_{}", Uuid::new_v4().simple()),
            room_id: room_id.to_string(),
            sender_id: sender_id.to_string(),
            content: content.to_string(),
            created_at: Utc::now(),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        };

        self.messages
            .write()
            .await
            .insert(message.id.clone(), message.clone());
        Ok(message)
    }

    async fn get(&self, id: &str) -> Result<Option<Message>, RepositoryError> {
        Ok(self.messages.read().await.get(id).cloned())
    }

    async fn list_by_room(&self, room_id: &str) -> Result<Vec<Message>, RepositoryError> {
        let mut messages = self
            .messages
            .read()
            .await
            .values()
            .filter(|message| message.room_id == room_id)
            .cloned()
            .collect::<Vec<_>>();
        messages.sort_by_key(|message| message.created_at);
        Ok(messages)
    }

    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        room_id: &str,
        sender_id: &str,
        content: &str,
    ) -> Result<Message, RepositoryError> {
        let message = Message {
            id: format!("msg_{}", Uuid::new_v4().simple()),
            room_id: room_id.to_string(),
            sender_id: sender_id.to_string(),
            content: content.to_string(),
            created_at: Utc::now(),
            tenant_id: Some(tenant_id.to_string()),
        };

        self.messages
            .write()
            .await
            .insert(message.id.clone(), message.clone());
        Ok(message)
    }

    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Message>, RepositoryError> {
        Ok(self
            .messages
            .read()
            .await
            .get(id)
            .filter(|msg| msg.tenant_id.as_deref() == Some(tenant_id))
            .cloned())
    }

    #[cfg(feature = "multi-tenant")]
    async fn list_by_room_tenant(
        &self,
        tenant_id: &str,
        room_id: &str,
    ) -> Result<Vec<Message>, RepositoryError> {
        let mut messages = self
            .messages
            .read()
            .await
            .values()
            .filter(|message| {
                message.room_id == room_id
                    && message.tenant_id.as_deref() == Some(tenant_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        messages.sort_by_key(|message| message.created_at);
        Ok(messages)
    }
}

#[cfg(test)]
#[derive(Debug, Default, Clone)]
struct InMemoryMemberRepository {
    members: Arc<RwLock<HashMap<String, Member>>>,
}

#[cfg(test)]
#[async_trait]
impl MemberRepository for InMemoryMemberRepository {
    async fn create(&self, member_type: &str, email: &str) -> Result<Member, RepositoryError> {
        let member = Member {
            id: format!("member_{}", Uuid::new_v4().simple()),
            member_type: member_type.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
            #[cfg(feature = "multi-tenant")]
            tenant_id: None,
        };
        self.members
            .write()
            .await
            .insert(member.id.clone(), member.clone());
        Ok(member)
    }

    async fn get(&self, id: &str) -> Result<Option<Member>, RepositoryError> {
        Ok(self.members.read().await.get(id).cloned())
    }

    #[cfg(feature = "multi-tenant")]
    async fn create_tenant(
        &self,
        tenant_id: &str,
        member_type: &str,
        email: &str,
    ) -> Result<Member, RepositoryError> {
        let member = Member {
            id: format!("member_{}", Uuid::new_v4().simple()),
            member_type: member_type.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
            tenant_id: Some(tenant_id.to_string()),
        };
        self.members
            .write()
            .await
            .insert(member.id.clone(), member.clone());
        Ok(member)
    }

    #[cfg(feature = "multi-tenant")]
    async fn get_tenant(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<Member>, RepositoryError> {
        Ok(self
            .members
            .read()
            .await
            .get(id)
            .filter(|member| member.tenant_id.as_deref() == Some(tenant_id))
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryMemberRepository, InMemoryMessageRepository, InMemoryRoomRepository,
        MemberRepository, MessageRepository, RoomRepository,
    };

    #[tokio::test]
    async fn room_repository_create_get_and_list() {
        let repository = InMemoryRoomRepository::default();

        let created = repository.create("general", Some("team")).await.unwrap();
        let loaded = repository.get(&created.id).await.unwrap().unwrap();

        assert_eq!(loaded.name, "general");
        assert_eq!(loaded.topic.as_deref(), Some("team"));

        let rooms = repository.list().await.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, created.id);
    }

    #[tokio::test]
    async fn message_repository_create_get_and_list_by_room() {
        let repository = InMemoryMessageRepository::default();

        let first = repository
            .create("room_1", "member_1", "hello")
            .await
            .unwrap();
        repository
            .create("room_2", "member_1", "skip")
            .await
            .unwrap();

        let loaded = repository.get(&first.id).await.unwrap().unwrap();
        assert_eq!(loaded.content, "hello");

        let room_messages = repository.list_by_room("room_1").await.unwrap();
        assert_eq!(room_messages.len(), 1);
        assert_eq!(room_messages[0].id, first.id);
    }

    #[tokio::test]
    async fn member_repository_create_and_get() {
        let repository = InMemoryMemberRepository::default();

        let created = repository
            .create("human", "alice@example.com")
            .await
            .unwrap();
        let loaded = repository.get(&created.id).await.unwrap().unwrap();

        assert_eq!(loaded.member_type, "human");
        assert_eq!(loaded.email, "alice@example.com");
    }

    #[cfg(feature = "multi-tenant")]
    #[tokio::test]
    async fn room_repository_tenant_isolation() {
        let repository = InMemoryRoomRepository::default();

        let tenant_a_room = repository
            .create_tenant("tenant_a", "room-a", Some("topic-a"))
            .await
            .unwrap();
        let tenant_b_room = repository
            .create_tenant("tenant_b", "room-b", Some("topic-b"))
            .await
            .unwrap();

        assert_eq!(
            repository.get_tenant("tenant_a", &tenant_a_room.id).await.unwrap(),
            Some(tenant_a_room.clone())
        );
        assert_eq!(
            repository.get_tenant("tenant_b", &tenant_b_room.id).await.unwrap(),
            Some(tenant_b_room.clone())
        );

        assert_eq!(
            repository.get_tenant("tenant_a", &tenant_b_room.id).await.unwrap(),
            None,
            "Cross-tenant access should return None"
        );
        assert_eq!(
            repository.get_tenant("tenant_b", &tenant_a_room.id).await.unwrap(),
            None,
            "Cross-tenant access should return None"
        );

        let tenant_a_rooms = repository.list_tenant("tenant_a").await.unwrap();
        assert_eq!(tenant_a_rooms.len(), 1);
        assert_eq!(tenant_a_rooms[0].name, "room-a");

        let tenant_b_rooms = repository.list_tenant("tenant_b").await.unwrap();
        assert_eq!(tenant_b_rooms.len(), 1);
        assert_eq!(tenant_b_rooms[0].name, "room-b");
    }

    #[cfg(feature = "multi-tenant")]
    #[tokio::test]
    async fn message_repository_tenant_isolation() {
        let repository = InMemoryMessageRepository::default();

        let tenant_a_msg = repository
            .create_tenant("tenant_a", "room_1", "sender_1", "hello from a")
            .await
            .unwrap();
        let tenant_b_msg = repository
            .create_tenant("tenant_b", "room_1", "sender_2", "hello from b")
            .await
            .unwrap();

        assert_eq!(
            repository.get_tenant("tenant_a", &tenant_a_msg.id).await.unwrap(),
            Some(tenant_a_msg.clone())
        );
        assert_eq!(
            repository.get_tenant("tenant_b", &tenant_b_msg.id).await.unwrap(),
            Some(tenant_b_msg.clone())
        );

        assert_eq!(
            repository.get_tenant("tenant_a", &tenant_b_msg.id).await.unwrap(),
            None,
            "Cross-tenant access should return None"
        );

        let tenant_a_messages = repository.list_by_room_tenant("tenant_a", "room_1").await.unwrap();
        assert_eq!(tenant_a_messages.len(), 1);
        assert_eq!(tenant_a_messages[0].content, "hello from a");

        let tenant_b_messages = repository.list_by_room_tenant("tenant_b", "room_1").await.unwrap();
        assert_eq!(tenant_b_messages.len(), 1);
        assert_eq!(tenant_b_messages[0].content, "hello from b");
    }

    #[cfg(feature = "multi-tenant")]
    #[tokio::test]
    async fn member_repository_tenant_isolation() {
        let repository = InMemoryMemberRepository::default();

        let tenant_a_member = repository
            .create_tenant("tenant_a", "human", "alice@tenant-a.com")
            .await
            .unwrap();
        let tenant_b_member = repository
            .create_tenant("tenant_b", "human", "bob@tenant-b.com")
            .await
            .unwrap();

        assert_eq!(
            repository.get_tenant("tenant_a", &tenant_a_member.id).await.unwrap(),
            Some(tenant_a_member.clone())
        );
        assert_eq!(
            repository.get_tenant("tenant_b", &tenant_b_member.id).await.unwrap(),
            Some(tenant_b_member.clone())
        );

        assert_eq!(
            repository.get_tenant("tenant_a", &tenant_b_member.id).await.unwrap(),
            None,
            "Cross-tenant access should return None"
        );
        assert_eq!(
            repository.get_tenant("tenant_b", &tenant_a_member.id).await.unwrap(),
            None,
            "Cross-tenant access should return None"
        );
    }

    #[cfg(feature = "multi-tenant")]
    #[tokio::test]
    async fn tenant_data_isolation_prevents_cross_tenant_access() {
        let room_repo = InMemoryRoomRepository::default();
        let msg_repo = InMemoryMessageRepository::default();

        let room_a = room_repo
            .create_tenant("tenant_evil", "secret-room", Some("confidential"))
            .await
            .unwrap();
        let msg_a = msg_repo
            .create_tenant("tenant_evil", &room_a.id, "attacker", "secret data")
            .await
            .unwrap();

        let room_result = room_repo.get_tenant("tenant_victim", &room_a.id).await.unwrap();
        assert_eq!(room_result, None, "Victim tenant should not see evil tenant's room");

        let msg_result = msg_repo.get_tenant("tenant_victim", &msg_a.id).await.unwrap();
        assert_eq!(msg_result, None, "Victim tenant should not see evil tenant's message");

        let victim_rooms = room_repo.list_tenant("tenant_victim").await.unwrap();
        assert!(victim_rooms.is_empty(), "Victim should have empty room list");

        let victim_messages = msg_repo.list_by_room_tenant("tenant_victim", &room_a.id).await.unwrap();
        assert!(victim_messages.is_empty(), "Victim should have empty message list for evil room");
    }
}
