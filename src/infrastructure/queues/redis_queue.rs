use crate::domain::entities::{Task, TaskResult};
use crate::domain::repositories::QueueRepository;
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Redis-based queue repository implementation
/// Provides queue operations using Redis as the message broker
pub struct RedisQueueRepository {
    client: Client,
    connection: Arc<Mutex<ConnectionManager>>,
}

impl RedisQueueRepository {
    /// Creates a new RedisQueueRepository
    ///
    /// # Arguments
    /// * `redis_url` - The Redis connection URL (e.g., "redis://localhost:6379")
    ///
    /// # Returns
    /// Result containing the new repository instance or an error
    ///
    /// # Errors
    /// Returns an error if the connection to Redis fails
    pub async fn new(redis_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = Client::open(redis_url)?;
        let connection = ConnectionManager::new(client.clone()).await?;

        Ok(Self {
            client,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Serializes a task to JSON
    ///
    /// # Arguments
    /// * `task` - The task to serialize
    ///
    /// # Returns
    /// Result containing the JSON string or an error
    fn serialize_task(task: &Task) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::to_string(task)?)
    }

    /// Deserializes a task from JSON
    ///
    /// # Arguments
    /// * `json` - The JSON string to deserialize
    ///
    /// # Returns
    /// Result containing the Task or an error
    fn deserialize_task(json: &str) -> Result<Task, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serializes a task result to JSON
    ///
    /// # Arguments
    /// * `result` - The task result to serialize
    ///
    /// # Returns
    /// Result containing the JSON string or an error
    fn serialize_result(result: &TaskResult) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::to_string(result)?)
    }
}

#[async_trait]
impl QueueRepository for RedisQueueRepository {
    async fn receive_task(
        &self,
        queue_name: &str,
        timeout_secs: Option<u64>,
    ) -> Result<Option<Task>, Box<dyn Error + Send + Sync>> {
        let mut conn = self.connection.lock().await;

        let result: Option<(String, String)> = if let Some(timeout) = timeout_secs {
            conn.brpop(queue_name, timeout as f64).await?
        } else {
            conn.brpop(queue_name, 0.0).await?
        };

        if let Some((_queue, task_json)) = result {
            let task = Self::deserialize_task(&task_json)?;
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    async fn send_result(
        &self,
        queue_name: &str,
        result: &TaskResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut conn = self.connection.lock().await;
        let result_json = Self::serialize_result(result)?;
        conn.lpush::<_, _, ()>(queue_name, result_json).await?;
        Ok(())
    }

    async fn acknowledge_task(
        &self,
        _task_id: &uuid::Uuid,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn reject_task(
        &self,
        _task_id: &uuid::Uuid,
        _requeue: bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let mut conn = self.connection.lock().await;
        let result: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(result == "PONG")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::task::TaskType;

    #[test]
    fn test_task_serialization() {
        let task = Task::new(TaskType::ExecuteCommand {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            working_dir: None,
            env_vars: None,
        });

        let json = RedisQueueRepository::serialize_task(&task).unwrap();
        let deserialized = RedisQueueRepository::deserialize_task(&json).unwrap();

        assert_eq!(task.id, deserialized.id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_connection() {
        let repo = RedisQueueRepository::new("redis://localhost:6379").await;
        assert!(repo.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_health_check() {
        let repo = RedisQueueRepository::new("redis://localhost:6379")
            .await
            .unwrap();
        let health = repo.health_check().await.unwrap();
        assert!(health);
    }
}
