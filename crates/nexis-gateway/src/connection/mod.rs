//! Connection management for Nexis Gateway

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Connection ID type
pub type ConnectionId = Uuid;

/// A connected client
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub member_id: String,
    pub room_id: Option<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// Connection manager
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<ConnectionId, Connection>>>,
    message_tx: broadcast::Sender<String>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        let (message_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
        }
    }

    /// Add a new connection
    pub async fn add_connection(&self, member_id: String) -> ConnectionId {
        let id = Uuid::new_v4();
        let connection = Connection {
            id,
            member_id,
            room_id: None,
            connected_at: chrono::Utc::now(),
        };

        let mut connections = self.connections.write().await;
        connections.insert(id, connection);

        tracing::info!("Connection {} added", id);
        id
    }

    /// Remove a connection
    pub async fn remove_connection(&self, id: ConnectionId) {
        let mut connections = self.connections.write().await;
        if connections.remove(&id).is_some() {
            tracing::info!("Connection {} removed", id);
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get message sender
    pub fn message_sender(&self) -> broadcast::Sender<String> {
        self.message_tx.clone()
    }

    /// Broadcast a message to all connections
    pub async fn broadcast(&self, message: String) {
        let _ = self.message_tx.send(message);
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connection_manager_tracks_connections() {
        let manager = ConnectionManager::new();
        
        assert_eq!(manager.connection_count().await, 0);
        
        let id1 = manager.add_connection("nexis:human:alice@example.com".to_string()).await;
        let id2 = manager.add_connection("nexis:ai:gpt-4".to_string()).await;
        
        assert_eq!(manager.connection_count().await, 2);
        
        manager.remove_connection(id1).await;
        assert_eq!(manager.connection_count().await, 1);
        
        manager.remove_connection(id2).await;
        assert_eq!(manager.connection_count().await, 0);
    }
}
