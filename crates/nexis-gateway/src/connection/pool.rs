//! High-performance connection management for 100K+ WebSocket connections
//!
//! Uses sharding and lock-free data structures for scalability.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::time::Instant;
use tokio::sync::{broadcast, OwnedSemaphorePermit, RwLock, Semaphore};
use uuid::Uuid;

use crate::metrics::{
    record_pool_connection_added, record_pool_connection_removed,
    record_pool_peak_if_higher, record_pool_message_sent, record_pool_message_dropped,
};

/// Default number of shards for connection pool
const DEFAULT_SHARD_COUNT: usize = 64;

/// Default max connections
const DEFAULT_MAX_CONNECTIONS: usize = 100_000;

/// Connection ID type
pub type ConnectionId = Uuid;

/// A connected client
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub member_id: String,
    pub room_id: Option<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: Instant,
}

/// Statistics for the connection pool
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub total_connections: usize,
    pub active_shards: usize,
    pub messages_sent: u64,
    pub messages_dropped: u64,
}

/// A single shard of the connection pool
struct ConnectionShard {
    connections: RwLock<HashMap<ConnectionId, Connection>>,
    permits: RwLock<HashMap<ConnectionId, OwnedSemaphorePermit>>,
}

impl ConnectionShard {
    fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            permits: RwLock::new(HashMap::new()),
        }
    }
}

/// High-performance connection manager with sharding
pub struct ShardedConnectionManager {
    shards: Vec<ConnectionShard>,
    shard_mask: usize,
    active_connections: AtomicUsize,
    connection_slots: Arc<Semaphore>,
    message_tx: broadcast::Sender<BroadcastMessage>,
    messages_sent: AtomicU64,
    messages_dropped: AtomicU64,
    peak_connections: AtomicUsize,
}

#[derive(Debug, Clone)]
pub struct BroadcastMessage {
    pub room_id: Option<String>,
    pub payload: String,
}

impl ShardedConnectionManager {
    /// Create a new connection manager with default settings (100K connections, 64 shards)
    pub fn new() -> Self {
        Self::with_config(DEFAULT_MAX_CONNECTIONS, DEFAULT_SHARD_COUNT)
    }

    /// Create a new connection manager with explicit max connection count
    pub fn with_max_connections(max_connections: usize) -> Self {
        Self::with_config(max_connections, DEFAULT_SHARD_COUNT)
    }

    /// Create a new connection manager with custom configuration
    pub fn with_config(max_connections: usize, shard_count: usize) -> Self {
        // Ensure shard count is power of 2 for efficient masking
        let shard_count = shard_count.next_power_of_two();
        let shard_mask = shard_count - 1;

        let shards: Vec<ConnectionShard> = (0..shard_count)
            .map(|_| ConnectionShard::new())
            .collect();

        let (message_tx, _) = broadcast::channel(10000);

        Self {
            shards,
            shard_mask,
            active_connections: AtomicUsize::new(0),
            connection_slots: Arc::new(Semaphore::new(max_connections)),
            message_tx,
            messages_sent: AtomicU64::new(0),
            messages_dropped: AtomicU64::new(0),
            peak_connections: AtomicUsize::new(0),
        }
    }

    /// Get the shard index for a connection ID
    #[inline]
    fn shard_index(&self, id: ConnectionId) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        (hasher.finish() as usize) & self.shard_mask
    }

    /// Try to add a new connection, returning None when the pool is saturated
    pub async fn try_add_connection(&self, member_id: String) -> Option<ConnectionId> {
        let permit = self.connection_slots.clone().try_acquire_owned().ok()?;
        let id = Uuid::new_v4();
        let shard_idx = self.shard_index(id);
        let connection = Connection {
            id,
            member_id,
            room_id: None,
            connected_at: chrono::Utc::now(),
            last_activity: Instant::now(),
        };

        {
            let shard = &self.shards[shard_idx];
            shard.connections.write().await.insert(id, connection);
            shard.permits.write().await.insert(id, permit);
        }

        let count = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        
        // Track peak connections
        loop {
            let peak = self.peak_connections.load(Ordering::Relaxed);
            if count <= peak || self.peak_connections.compare_exchange_weak(
                peak,
                count,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ).is_ok() {
                break;
            }
        }

        // Record metrics
        record_pool_connection_added(count);
        record_pool_peak_if_higher(count);

        tracing::debug!(
            connection_id = %id,
            shard = shard_idx,
            total = count,
            "Connection added"
        );

        Some(id)
    }

    /// Add a new connection (panics if pool is saturated)
    pub async fn add_connection(&self, member_id: String) -> ConnectionId {
        self.try_add_connection(member_id)
            .await
            .expect("connection pool saturated")
    }

    /// Remove a connection
    pub async fn remove_connection(&self, id: ConnectionId) {
        let shard_idx = self.shard_index(id);
        let shard = &self.shards[shard_idx];

        let mut connections = shard.connections.write().await;
        if connections.remove(&id).is_some() {
            drop(connections);
            let mut permits = shard.permits.write().await;
            permits.remove(&id);
            
            let count = self.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
            
            // Record metrics
            record_pool_connection_removed(count);
            
            tracing::debug!(
                connection_id = %id,
                shard = shard_idx,
                total = count,
                "Connection removed"
            );
        }
    }

    /// Get a connection by ID
    pub async fn get_connection(&self, id: ConnectionId) -> Option<Connection> {
        let shard_idx = self.shard_index(id);
        let shard = &self.shards[shard_idx];
        shard.connections.read().await.get(&id).cloned()
    }

    /// Update connection's room assignment
    pub async fn set_room(&self, id: ConnectionId, room_id: Option<String>) -> bool {
        let shard_idx = self.shard_index(id);
        let shard = &self.shards[shard_idx];
        
        let mut connections = shard.connections.write().await;
        if let Some(conn) = connections.get_mut(&id) {
            conn.room_id = room_id;
            conn.last_activity = Instant::now();
            true
        } else {
            false
        }
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Get peak connection count
    pub fn peak_connection_count(&self) -> usize {
        self.peak_connections.load(Ordering::Relaxed)
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let total = self.active_connections.load(Ordering::Relaxed);
        let active_shards = self.shards.iter().filter(|shard| {
            // Non-blocking check - might be slightly inaccurate
            shard.connections.try_read().map(|g| !g.is_empty()).unwrap_or(false)
        }).count();

        PoolStats {
            total_connections: total,
            active_shards,
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(Ordering::Relaxed),
        }
    }

    /// Subscribe to broadcast messages
    pub fn subscribe(&self) -> broadcast::Receiver<BroadcastMessage> {
        self.message_tx.subscribe()
    }

    /// Broadcast a message to all connections (or room-specific)
    pub async fn broadcast(&self, room_id: Option<String>, message: String) {
        let msg = BroadcastMessage {
            room_id,
            payload: message,
        };
        
        let receiver_count = self.message_tx.receiver_count();
        if receiver_count > 0 {
            if self.message_tx.send(msg).is_err() {
                self.messages_dropped.fetch_add(1, Ordering::Relaxed);
                record_pool_message_dropped();
            } else {
                self.messages_sent.fetch_add(1, Ordering::Relaxed);
                record_pool_message_sent();
            }
        }
    }

    /// Get all connection IDs (for debugging/admin)
    pub async fn all_connection_ids(&self) -> Vec<ConnectionId> {
        let mut ids = Vec::new();
        for shard in &self.shards {
            let connections = shard.connections.read().await;
            ids.extend(connections.keys().copied());
        }
        ids
    }

    /// Get connections count per shard (for load balancing diagnostics)
    pub async fn shard_distribution(&self) -> Vec<usize> {
        let mut distribution = Vec::with_capacity(self.shards.len());
        for shard in &self.shards {
            distribution.push(shard.connections.read().await.len());
        }
        distribution
    }

    /// Clean up inactive connections (optional maintenance)
    pub async fn cleanup_inactive(&self, max_idle_secs: u64) -> usize {
        let now = Instant::now();
        let mut removed = 0;

        for (shard_idx, shard) in self.shards.iter().enumerate() {
            let mut connections = shard.connections.write().await;
            let inactive: Vec<ConnectionId> = connections
                .iter()
                .filter(|(_, conn)| {
                    now.duration_since(conn.last_activity).as_secs() > max_idle_secs
                })
                .map(|(id, _)| *id)
                .collect();

            for id in inactive {
                connections.remove(&id);
                let mut permits = shard.permits.write().await;
                permits.remove(&id);
                self.active_connections.fetch_sub(1, Ordering::Relaxed);
                removed += 1;
                tracing::debug!(
                    connection_id = %id,
                    shard = shard_idx,
                    "Removed inactive connection"
                );
            }
        }

        removed
    }
}

impl Default for ShardedConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sharded_manager_tracks_connections() {
        let manager = ShardedConnectionManager::new();
        assert_eq!(manager.connection_count(), 0);

        let id1 = manager.add_connection("alice".to_string()).await;
        let id2 = manager.add_connection("bob".to_string()).await;
        assert_eq!(manager.connection_count(), 2);

        manager.remove_connection(id1).await;
        assert_eq!(manager.connection_count(), 1);

        manager.remove_connection(id2).await;
        assert_eq!(manager.connection_count(), 0);
    }

    #[tokio::test]
    async fn sharded_manager_enforces_pool_limit() {
        let manager = ShardedConnectionManager::with_max_connections(2);

        let first = manager.try_add_connection("alice".to_string()).await;
        let second = manager.try_add_connection("bob".to_string()).await;
        let third = manager.try_add_connection("charlie".to_string()).await;

        assert!(first.is_some());
        assert!(second.is_some());
        assert!(third.is_none());
        assert_eq!(manager.connection_count(), 2);
    }

    #[tokio::test]
    async fn sharded_manager_tracks_peak_connections() {
        let manager = ShardedConnectionManager::with_max_connections(10);

        let id1 = manager.add_connection("alice".to_string()).await;
        let id2 = manager.add_connection("bob".to_string()).await;
        assert_eq!(manager.peak_connection_count(), 2);

        manager.remove_connection(id1).await;
        manager.remove_connection(id2).await;
        assert_eq!(manager.connection_count(), 0);
        assert_eq!(manager.peak_connection_count(), 2); // Peak stays at 2
    }

    #[tokio::test]
    async fn sharded_manager_gets_connection() {
        let manager = ShardedConnectionManager::new();
        let id = manager.add_connection("alice".to_string()).await;

        let conn = manager.get_connection(id).await.unwrap();
        assert_eq!(conn.member_id, "alice");
        assert!(conn.room_id.is_none());
    }

    #[tokio::test]
    async fn sharded_manager_sets_room() {
        let manager = ShardedConnectionManager::new();
        let id = manager.add_connection("alice".to_string()).await;

        manager.set_room(id, Some("room_123".to_string())).await;
        
        let conn = manager.get_connection(id).await.unwrap();
        assert_eq!(conn.room_id, Some("room_123".to_string()));
    }

    #[tokio::test]
    async fn sharded_manager_distributes_across_shards() {
        let manager = ShardedConnectionManager::with_config(1000, 16);

        // Add many connections
        for i in 0..100 {
            manager.add_connection(format!("user_{}", i)).await;
        }

        let distribution = manager.shard_distribution().await;
        
        // All shards should have roughly similar load (not exact due to hash distribution)
        let total: usize = distribution.iter().sum();
        assert_eq!(total, 100);
        
        // At least some shards should have connections
        let non_empty = distribution.iter().filter(|&&c| c > 0).count();
        assert!(non_empty > 1, "Connections should be distributed across shards");
    }

    #[tokio::test]
    async fn sharded_manager_stats() {
        let manager = ShardedConnectionManager::new();
        
        manager.add_connection("alice".to_string()).await;
        manager.add_connection("bob".to_string()).await;

        let stats = manager.stats();
        assert_eq!(stats.total_connections, 2);
    }

    #[tokio::test]
    async fn sharded_manager_broadcast() {
        let manager = ShardedConnectionManager::new();
        let mut rx = manager.subscribe();

        manager.broadcast(Some("room_1".to_string()), "hello".to_string()).await;

        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.room_id, Some("room_1".to_string()));
        assert_eq!(msg.payload, "hello");
    }
}
