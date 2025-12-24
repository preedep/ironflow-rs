use crate::domain::entities::{Task, TaskResult};
use async_trait::async_trait;
use std::error::Error;

/// Repository trait for queue operations
/// This abstraction allows different queue implementations (Redis, RabbitMQ, etc.)
/// to be used interchangeably without changing the core business logic
#[async_trait]
pub trait QueueRepository: Send + Sync {
    /// Receives a task from the queue
    ///
    /// # Arguments
    /// * `queue_name` - The name of the queue to receive from
    /// * `timeout_secs` - Optional timeout in seconds
    ///
    /// # Returns
    /// Result containing an optional Task if available, or an error
    ///
    /// # Errors
    /// Returns an error if the queue operation fails
    async fn receive_task(
        &self,
        queue_name: &str,
        timeout_secs: Option<u64>,
    ) -> Result<Option<Task>, Box<dyn Error + Send + Sync>>;

    /// Sends a task result to the queue
    ///
    /// # Arguments
    /// * `queue_name` - The name of the queue to send to
    /// * `result` - The task result to send
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the queue operation fails
    async fn send_result(
        &self,
        queue_name: &str,
        result: &TaskResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Acknowledges a task has been processed
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to acknowledge
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the acknowledgment fails
    async fn acknowledge_task(
        &self,
        task_id: &uuid::Uuid,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Rejects a task and optionally requeues it
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to reject
    /// * `requeue` - Whether to requeue the task
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the rejection fails
    async fn reject_task(
        &self,
        task_id: &uuid::Uuid,
        requeue: bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Checks if the queue connection is healthy
    ///
    /// # Returns
    /// Result containing true if healthy, false otherwise
    ///
    /// # Errors
    /// Returns an error if the health check fails
    async fn health_check(&self) -> Result<bool, Box<dyn Error + Send + Sync>>;
}
