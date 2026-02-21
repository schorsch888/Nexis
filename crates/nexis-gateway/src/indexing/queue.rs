//! Background task queue for async indexing operations

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::service::{IndexingError, IndexingService};

/// Indexing task to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexTask {
    /// Unique task ID
    pub id: Uuid,
    /// Message content to index
    pub message: String,
    /// Room ID
    pub room_id: Uuid,
    /// Custom metadata
    pub metadata: serde_json::Value,
    /// Number of retry attempts
    pub attempts: u32,
    /// Maximum retries before giving up
    pub max_retries: u32,
}

impl IndexTask {
    /// Create a new indexing task
    pub fn new(message: String, room_id: Uuid, metadata: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            message,
            room_id,
            metadata,
            attempts: 0,
            max_retries: 3,
        }
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Increment retry count
    pub fn increment_attempt(&mut self) {
        self.attempts += 1;
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_retries
    }
}

/// Task status for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending processing
    Pending,
    /// Task is currently being processed
    Processing,
    /// Task completed successfully
    Completed,
    /// Task failed permanently
    Failed,
}

/// Task queue statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    /// Number of pending tasks
    pub pending: usize,
    /// Number of completed tasks
    pub completed: u64,
    /// Number of failed tasks
    pub failed: u64,
    /// Number of retries
    pub retries: u64,
}

/// Background task queue for indexing
pub struct IndexingQueue {
    sender: mpsc::Sender<IndexTask>,
    stats: Arc<Mutex<QueueStats>>,
    pending_tasks: Arc<Mutex<HashMap<Uuid, IndexTask>>>,
}

impl IndexingQueue {
    /// Create a new indexing queue with the given service
    pub fn new(service: Arc<dyn IndexingService>, buffer_size: usize) -> Self {
        let (sender, mut receiver) = mpsc::channel::<IndexTask>(buffer_size);
        let stats = Arc::new(Mutex::new(QueueStats::default()));
        let pending_tasks = Arc::new(Mutex::new(HashMap::new()));

        let stats_clone = stats.clone();
        let pending_clone = pending_tasks.clone();
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            while let Some(mut task) = receiver.recv().await {
                debug!(task_id = %task.id, attempt = task.attempts, "Processing indexing task");

                match service.index_message(&task.message, task.room_id, task.metadata.clone()).await {
                    Ok(doc_id) => {
                        info!(task_id = %task.id, doc_id = %doc_id, "Indexing task completed");
                        let mut stats = stats_clone.lock().await;
                        stats.completed += 1;
                        let mut pending = pending_clone.lock().await;
                        pending.remove(&task.id);
                    }
                    Err(IndexingError::EmbeddingError(e)) => {
                        warn!(task_id = %task.id, error = %e, "Embedding error, will retry");
                        task.increment_attempt();
                        if task.can_retry() {
                            let mut stats = stats_clone.lock().await;
                            stats.retries += 1;
                            drop(stats);
                            
                            if let Err(e) = sender_clone.send(task.clone()).await {
                                error!(task_id = %task.id, error = %e, "Failed to re-queue task for retry");
                                let mut pending = pending_clone.lock().await;
                                pending.remove(&task.id);
                                let mut stats = stats_clone.lock().await;
                                stats.failed += 1;
                            } else {
                                let mut pending = pending_clone.lock().await;
                                pending.insert(task.id, task);
                            }
                        } else {
                            error!(task_id = %task.id, "Task failed after max retries");
                            let mut stats = stats_clone.lock().await;
                            stats.failed += 1;
                            let mut pending = pending_clone.lock().await;
                            pending.remove(&task.id);
                        }
                    }
                    Err(e) => {
                        error!(task_id = %task.id, error = %e, "Indexing task failed");
                        let mut stats = stats_clone.lock().await;
                        stats.failed += 1;
                        let mut pending = pending_clone.lock().await;
                        pending.remove(&task.id);
                    }
                }
            }
            debug!("Indexing queue processor stopped");
        });

        Self {
            sender,
            stats,
            pending_tasks,
        }
    }

    /// Enqueue a task for background processing
    pub async fn enqueue(&self, task: IndexTask) -> Result<(), IndexingError> {
        let task_id = task.id;
        self.sender
            .send(task.clone())
            .await
            .map_err(|_| IndexingError::InvalidMessage("Queue channel closed".to_string()))?;

        let mut pending = self.pending_tasks.lock().await;
        pending.insert(task_id, task);

        Ok(())
    }

    /// Enqueue a message for indexing
    pub async fn index_message(
        &self,
        message: String,
        room_id: Uuid,
        metadata: serde_json::Value,
    ) -> Result<Uuid, IndexingError> {
        let task = IndexTask::new(message, room_id, metadata);
        let task_id = task.id;
        self.enqueue(task).await?;
        Ok(task_id)
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let stats = self.stats.lock().await.clone();
        let pending = self.pending_tasks.lock().await.len();
        QueueStats { pending, ..stats }
    }
}

/// Synchronous task queue for non-async contexts
pub struct SyncIndexingQueue {
    tasks: Arc<Mutex<VecDeque<IndexTask>>>,
    stats: Arc<Mutex<QueueStats>>,
}

impl SyncIndexingQueue {
    /// Create a new synchronous queue
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(Mutex::new(QueueStats::default())),
        }
    }

    /// Push a task to the queue
    pub async fn push(&self, task: IndexTask) {
        let mut tasks = self.tasks.lock().await;
        tasks.push_back(task);
    }

    /// Pop a task from the queue
    pub async fn pop(&self) -> Option<IndexTask> {
        let mut tasks = self.tasks.lock().await;
        tasks.pop_front()
    }

    /// Get queue length
    pub async fn len(&self) -> usize {
        self.tasks.lock().await.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.tasks.lock().await.is_empty()
    }

    /// Get statistics
    pub async fn stats(&self) -> QueueStats {
        let stats = self.stats.lock().await.clone();
        let pending = self.tasks.lock().await.len();
        QueueStats { pending, ..stats }
    }
}

impl Default for SyncIndexingQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_task_creation() {
        let room_id = Uuid::new_v4();
        let task = IndexTask::new("Hello world".to_string(), room_id, serde_json::json!({}));

        assert!(!task.message.is_empty());
        assert_eq!(task.room_id, room_id);
        assert_eq!(task.attempts, 0);
    }

    #[test]
    fn index_task_retry_logic() {
        let mut task = IndexTask::new("Test".to_string(), Uuid::new_v4(), serde_json::json!({}));
        task.max_retries = 2;

        assert!(task.can_retry());
        task.increment_attempt();
        assert!(task.can_retry());
        task.increment_attempt();
        assert!(!task.can_retry());
    }

    #[tokio::test]
    async fn sync_queue_operations() {
        let queue = SyncIndexingQueue::new();

        assert!(queue.is_empty().await);

        let task = IndexTask::new("Test".to_string(), Uuid::new_v4(), serde_json::json!({}));
        queue.push(task).await;

        assert_eq!(queue.len().await, 1);
        assert!(!queue.is_empty().await);

        let popped = queue.pop().await;
        assert!(popped.is_some());
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn queue_stats() {
        let queue = SyncIndexingQueue::new();
        let stats = queue.stats().await;

        assert_eq!(stats.pending, 0);
        assert_eq!(stats.completed, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.retries, 0);
    }

    #[test]
    fn retry_respects_max_retries() {
        let mut task = IndexTask::new("Test".to_string(), Uuid::new_v4(), serde_json::json!({}));
        task.max_retries = 3;

        assert!(task.can_retry());
        task.increment_attempt();
        assert_eq!(task.attempts, 1);
        assert!(task.can_retry());
        
        task.increment_attempt();
        task.increment_attempt();
        assert_eq!(task.attempts, 3);
        assert!(!task.can_retry());
    }
}
