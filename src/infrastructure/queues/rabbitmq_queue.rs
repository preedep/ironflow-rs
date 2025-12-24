use crate::domain::entities::{Task, TaskResult};
use crate::domain::repositories::QueueRepository;
use async_trait::async_trait;
use futures::StreamExt;
use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties,
};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

/// RabbitMQ-based queue repository implementation
/// Provides queue operations using RabbitMQ as the message broker
pub struct RabbitMqQueueRepository {
    connection: Connection,
    channel: Arc<Mutex<Channel>>,
    pending_acks: Arc<Mutex<HashMap<uuid::Uuid, u64>>>,
}

impl RabbitMqQueueRepository {
    /// Creates a new RabbitMqQueueRepository
    ///
    /// # Arguments
    /// * `amqp_url` - The AMQP connection URL (e.g., "amqp://localhost:5672")
    /// * `prefetch_count` - Number of messages to prefetch
    ///
    /// # Returns
    /// Result containing the new repository instance or an error
    ///
    /// # Errors
    /// Returns an error if the connection to RabbitMQ fails
    pub async fn new(
        amqp_url: &str,
        prefetch_count: u16,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let connection = Connection::connect(amqp_url, ConnectionProperties::default()).await?;
        let channel = connection.create_channel().await?;

        channel
            .basic_qos(prefetch_count, BasicQosOptions::default())
            .await?;

        Ok(Self {
            connection,
            channel: Arc::new(Mutex::new(channel)),
            pending_acks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Declares a queue if it doesn't exist
    ///
    /// # Arguments
    /// * `queue_name` - The name of the queue to declare
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the queue declaration fails
    async fn declare_queue(&self, queue_name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let channel = self.channel.lock().await;
        channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;
        Ok(())
    }

    /// Serializes a task to JSON
    ///
    /// # Arguments
    /// * `task` - The task to serialize
    ///
    /// # Returns
    /// Result containing the JSON bytes or an error
    fn serialize_task(task: &Task) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::to_vec(task)?)
    }

    /// Deserializes a task from JSON
    ///
    /// # Arguments
    /// * `data` - The JSON bytes to deserialize
    ///
    /// # Returns
    /// Result containing the Task or an error
    fn deserialize_task(data: &[u8]) -> Result<Task, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Serializes a task result to JSON
    ///
    /// # Arguments
    /// * `result` - The task result to serialize
    ///
    /// # Returns
    /// Result containing the JSON bytes or an error
    fn serialize_result(result: &TaskResult) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        Ok(serde_json::to_vec(result)?)
    }
}

#[async_trait]
impl QueueRepository for RabbitMqQueueRepository {
    async fn receive_task(
        &self,
        queue_name: &str,
        _timeout_secs: Option<u64>,
    ) -> Result<Option<Task>, Box<dyn Error + Send + Sync>> {
        self.declare_queue(queue_name).await?;

        let channel = self.channel.lock().await;
        let mut consumer = channel
            .basic_consume(
                queue_name,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        drop(channel);

        if let Some(delivery) = consumer.next().await {
            let delivery = delivery?;
            let task = Self::deserialize_task(&delivery.data)?;

            let mut pending = self.pending_acks.lock().await;
            pending.insert(task.id, delivery.delivery_tag);

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
        self.declare_queue(queue_name).await?;

        let channel = self.channel.lock().await;
        let payload = Self::serialize_result(result)?;

        channel
            .basic_publish(
                "",
                queue_name,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default().with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    async fn acknowledge_task(
        &self,
        task_id: &uuid::Uuid,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut pending = self.pending_acks.lock().await;

        if let Some(delivery_tag) = pending.remove(task_id) {
            let channel = self.channel.lock().await;
            channel
                .basic_ack(delivery_tag, BasicAckOptions::default())
                .await?;
        }

        Ok(())
    }

    async fn reject_task(
        &self,
        task_id: &uuid::Uuid,
        requeue: bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut pending = self.pending_acks.lock().await;

        if let Some(delivery_tag) = pending.remove(task_id) {
            let channel = self.channel.lock().await;
            channel
                .basic_nack(
                    delivery_tag,
                    BasicNackOptions {
                        requeue,
                        ..Default::default()
                    },
                )
                .await?;
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        Ok(self.connection.status().connected())
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

        let bytes = RabbitMqQueueRepository::serialize_task(&task).unwrap();
        let deserialized = RabbitMqQueueRepository::deserialize_task(&bytes).unwrap();

        assert_eq!(task.id, deserialized.id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_rabbitmq_connection() {
        let repo = RabbitMqQueueRepository::new("amqp://localhost:5672", 10).await;
        assert!(repo.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_health_check() {
        let repo = RabbitMqQueueRepository::new("amqp://localhost:5672", 10)
            .await
            .unwrap();
        let health = repo.health_check().await.unwrap();
        assert!(health);
    }
}
