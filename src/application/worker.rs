use crate::application::TaskProcessor;
use crate::domain::entities::{Task, TaskResult};
use crate::domain::repositories::QueueRepository;
use crate::domain::value_objects::WorkerConfig;
use crate::infrastructure::logging::{LogBuilder, LogLevel};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};

/// Worker that processes tasks from a queue
/// This is the main orchestrator that pulls tasks and delegates to the processor
pub struct Worker {
    config: WorkerConfig,
    queue_repository: Arc<dyn QueueRepository>,
    task_processor: Arc<TaskProcessor>,
    log_builder: LogBuilder,
    shutdown_signal: Arc<tokio::sync::Notify>,
}

impl Worker {
    /// Creates a new Worker
    ///
    /// # Arguments
    /// * `config` - Worker configuration
    /// * `queue_repository` - Queue repository for receiving tasks and sending results
    /// * `task_processor` - Task processor for executing tasks
    /// * `log_builder` - Log builder for structured logging
    ///
    /// # Returns
    /// A new Worker instance
    pub fn new(
        config: WorkerConfig,
        queue_repository: Arc<dyn QueueRepository>,
        task_processor: Arc<TaskProcessor>,
        log_builder: LogBuilder,
    ) -> Self {
        Self {
            config,
            queue_repository,
            task_processor,
            log_builder,
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Starts the worker and begins processing tasks
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the worker fails to start or encounters a fatal error
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_builder
            .build(
                LogLevel::Info,
                format!("Starting worker: {}", self.config.worker_id),
            )
            .with_code_location("worker::start".to_string())
            .log();

        let semaphore = Arc::new(Semaphore::new(self.config.worker.concurrency));
        let mut health_check_interval = interval(Duration::from_secs(
            self.config.worker.health_check_interval_secs,
        ));

        let shutdown_signal = self.shutdown_signal.clone();
        let shutdown_timeout = self.config.worker.shutdown_timeout_secs;

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            shutdown_signal.notify_waiters();
        });

        loop {
            tokio::select! {
                _ = self.shutdown_signal.notified() => {
                    self.log_builder
                        .build(
                            LogLevel::Info,
                            "Shutdown signal received, waiting for tasks to complete".to_string(),
                        )
                        .with_code_location("worker::start".to_string())
                        .log();

                    tokio::time::timeout(
                        Duration::from_secs(shutdown_timeout),
                        async {
                            let _ = semaphore.acquire_many(self.config.worker.concurrency as u32).await;
                        }
                    ).await.ok();

                    self.log_builder
                        .build(LogLevel::Info, "Worker stopped".to_string())
                        .with_code_location("worker::start".to_string())
                        .log();

                    break;
                }

                _ = health_check_interval.tick() => {
                    self.perform_health_check().await;
                }

                _ = async {
                    let permit = semaphore.clone().acquire_owned().await.ok();
                    if let Some(permit) = permit {
                        self.process_next_task(permit).await;
                    }
                } => {}
            }
        }

        Ok(())
    }

    /// Processes the next task from the queue
    ///
    /// # Arguments
    /// * `permit` - Semaphore permit for concurrency control
    async fn process_next_task(&self, permit: tokio::sync::OwnedSemaphorePermit) {
        let queue_name = match &self.config.queue {
            crate::domain::value_objects::QueueConfig::Redis { task_queue, .. } => task_queue.clone(),
            crate::domain::value_objects::QueueConfig::RabbitMq { task_queue, .. } => task_queue.clone(),
        };

        let result_queue = match &self.config.queue {
            crate::domain::value_objects::QueueConfig::Redis { result_queue, .. } => result_queue.clone(),
            crate::domain::value_objects::QueueConfig::RabbitMq { result_queue, .. } => result_queue.clone(),
        };

        let queue_repo = self.queue_repository.clone();
        let processor = self.task_processor.clone();
        let log_builder = self.log_builder.clone();

        tokio::spawn(async move {
            let _permit = permit;

            match queue_repo
                .receive_task(&queue_name, Some(30))
                .await
            {
                Ok(Some(task)) => {
                    log_builder
                        .build(
                            LogLevel::Info,
                            format!("Received task: {}", task.id),
                        )
                        .with_correlation_id(task.correlation_id.clone().unwrap_or_default())
                        .with_code_location("worker::process_next_task".to_string())
                        .log();

                    let task_id = task.id;
                    let result = processor.process_task(task.clone()).await;

                    match result {
                        Ok(task_result) => {
                            if let Some(callback_url) = &task.callback_url {
                                Self::send_callback(callback_url, &task_result).await.ok();
                            }

                            if task.result_queue.is_some() || !result_queue.is_empty() {
                                let target_queue = task.result_queue.as_ref().unwrap_or(&result_queue);
                                queue_repo
                                    .send_result(target_queue, &task_result)
                                    .await
                                    .ok();
                            }

                            queue_repo.acknowledge_task(&task_id).await.ok();

                            log_builder
                                .build(
                                    LogLevel::Info,
                                    format!("Task completed successfully: {}", task_id),
                                )
                                .with_correlation_id(task.correlation_id.clone().unwrap_or_default())
                                .with_code_location("worker::process_next_task".to_string())
                                .log();
                        }
                        Err(e) => {
                            log_builder
                                .build(
                                    LogLevel::Error,
                                    format!("Task processing failed: {} - {}", task_id, e),
                                )
                                .with_correlation_id(task.correlation_id.clone().unwrap_or_default())
                                .with_code_location("worker::process_next_task".to_string())
                                .log();

                            let should_retry = task.can_retry();
                            queue_repo.reject_task(&task_id, should_retry).await.ok();
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    log_builder
                        .build(
                            LogLevel::Error,
                            format!("Failed to receive task: {}", e),
                        )
                        .with_code_location("worker::process_next_task".to_string())
                        .log();
                }
            }
        });
    }

    /// Sends a callback with the task result
    ///
    /// # Arguments
    /// * `url` - The callback URL
    /// * `result` - The task result to send
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn send_callback(
        url: &str,
        result: &TaskResult,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        client.post(url).json(result).send().await?;
        Ok(())
    }

    /// Performs a health check on the queue connection
    async fn perform_health_check(&self) {
        match self.queue_repository.health_check().await {
            Ok(true) => {
                self.log_builder
                    .build(LogLevel::Debug, "Health check passed".to_string())
                    .with_code_location("worker::perform_health_check".to_string())
                    .log();
            }
            Ok(false) | Err(_) => {
                self.log_builder
                    .build(LogLevel::Warn, "Health check failed".to_string())
                    .with_code_location("worker::perform_health_check".to_string())
                    .log();
            }
        }
    }

    /// Triggers a graceful shutdown of the worker
    pub fn shutdown(&self) {
        self.shutdown_signal.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_creation() {
        use crate::domain::repositories::QueueRepository;
        use crate::domain::services::{CommandExecutor, FileTransferService, FileWatcherService};
        use crate::infrastructure::services::{
            DefaultCommandExecutor, NotifyFileWatcherService, OpenDalFileTransferService,
        };
        use async_trait::async_trait;

        struct MockQueueRepo;

        #[async_trait]
        impl QueueRepository for MockQueueRepo {
            async fn receive_task(
                &self,
                _queue_name: &str,
                _timeout_secs: Option<u64>,
            ) -> Result<Option<Task>, Box<dyn std::error::Error + Send + Sync>> {
                Ok(None)
            }

            async fn send_result(
                &self,
                _queue_name: &str,
                _result: &TaskResult,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Ok(())
            }

            async fn acknowledge_task(
                &self,
                _task_id: &uuid::Uuid,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Ok(())
            }

            async fn reject_task(
                &self,
                _task_id: &uuid::Uuid,
                _requeue: bool,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Ok(())
            }

            async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
                Ok(true)
            }
        }

        let config = WorkerConfig::default();
        let queue_repo = Arc::new(MockQueueRepo) as Arc<dyn QueueRepository>;
        let executor = Arc::new(DefaultCommandExecutor::new()) as Arc<dyn CommandExecutor>;
        let transfer = Arc::new(OpenDalFileTransferService::new()) as Arc<dyn FileTransferService>;
        let watcher = Arc::new(NotifyFileWatcherService::new()) as Arc<dyn FileWatcherService>;
        let log_builder = LogBuilder::new();

        let processor = Arc::new(TaskProcessor::new(
            executor,
            transfer,
            watcher,
            None,
            log_builder.clone(),
        ));

        let _worker = Worker::new(config, queue_repo, processor, log_builder);
    }
}
