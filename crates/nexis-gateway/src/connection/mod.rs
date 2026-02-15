//! Connection management for Nexis Gateway

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{broadcast, OwnedSemaphorePermit, RwLock, Semaphore};
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
    connection_permits: Arc<RwLock<HashMap<ConnectionId, OwnedSemaphorePermit>>>,
    active_connections: Arc<AtomicUsize>,
    connection_slots: Arc<Semaphore>,
    message_tx: broadcast::Sender<String>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self::with_max_connections(10_000)
    }

    /// Create a new connection manager with an explicit max connection count.
    pub fn with_max_connections(max_connections: usize) -> Self {
        let (message_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_permits: Arc::new(RwLock::new(HashMap::new())),
            active_connections: Arc::new(AtomicUsize::new(0)),
            connection_slots: Arc::new(Semaphore::new(max_connections)),
            message_tx,
        }
    }

    /// Try to add a new connection, returning None when the pool is saturated.
    pub async fn try_add_connection(&self, member_id: String) -> Option<ConnectionId> {
        let permit = self.connection_slots.clone().try_acquire_owned().ok()?;
        let id = Uuid::new_v4();
        let connection = Connection {
            id,
            member_id,
            room_id: None,
            connected_at: chrono::Utc::now(),
        };

        {
            let mut connections = self.connections.write().await;
            connections.insert(id, connection);
        }
        {
            let mut permits = self.connection_permits.write().await;
            permits.insert(id, permit);
        }

        self.active_connections.fetch_add(1, Ordering::Relaxed);
        tracing::info!("Connection {} added", id);
        Some(id)
    }

    /// Add a new connection
    pub async fn add_connection(&self, member_id: String) -> ConnectionId {
        self.try_add_connection(member_id)
            .await
            .expect("connection pool saturated")
    }

    /// Remove a connection
    pub async fn remove_connection(&self, id: ConnectionId) {
        let mut connections = self.connections.write().await;
        if connections.remove(&id).is_some() {
            drop(connections);
            let mut permits = self.connection_permits.write().await;
            permits.remove(&id);
            self.active_connections.fetch_sub(1, Ordering::Relaxed);
            tracing::info!("Connection {} removed", id);
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
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

    #[tokio::test]
    async fn connection_manager_enforces_pool_limit() {
        let manager = ConnectionManager::with_max_connections(1);

        let first = manager
            .try_add_connection("nexis:human:alice@example.com".to_string())
            .await;
        assert!(first.is_some());
        assert_eq!(manager.connection_count().await, 1);

        let second = manager
            .try_add_connection("nexis:ai:gpt-4".to_string())
            .await;
        assert!(second.is_none());
        assert_eq!(manager.connection_count().await, 1);
    }
}
