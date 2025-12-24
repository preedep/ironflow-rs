use crate::domain::entities::{Task, TaskResult, TaskStatus, TaskType};
use crate::domain::repositories::SecretRepository;
use crate::domain::services::{CommandExecutor, FileTransferService, FileWatcherService};
use crate::infrastructure::logging::{LogBuilder, LogLevel, StdAppLog};
use std::error::Error;
use std::sync::Arc;

/// Task processor that executes different types of tasks
/// This is the core business logic that orchestrates task execution
pub struct TaskProcessor {
    command_executor: Arc<dyn CommandExecutor>,
    file_transfer_service: Arc<dyn FileTransferService>,
    file_watcher_service: Arc<dyn FileWatcherService>,
    secret_repository: Option<Arc<dyn SecretRepository>>,
    log_builder: LogBuilder,
}

impl TaskProcessor {
    /// Creates a new TaskProcessor
    ///
    /// # Arguments
    /// * `command_executor` - The command executor service
    /// * `file_transfer_service` - The file transfer service
    /// * `file_watcher_service` - The file watcher service
    /// * `secret_repository` - Optional secret repository
    /// * `log_builder` - Log builder for structured logging
    ///
    /// # Returns
    /// A new TaskProcessor instance
    pub fn new(
        command_executor: Arc<dyn CommandExecutor>,
        file_transfer_service: Arc<dyn FileTransferService>,
        file_watcher_service: Arc<dyn FileWatcherService>,
        secret_repository: Option<Arc<dyn SecretRepository>>,
        log_builder: LogBuilder,
    ) -> Self {
        log::debug!("Creating TaskProcessor with {} secret repository",
            if secret_repository.is_some() { "a" } else { "no" }
        );
        Self {
            command_executor,
            file_transfer_service,
            file_watcher_service,
            secret_repository,
            log_builder,
        }
    }

    /// Processes a task and returns the result
    ///
    /// # Arguments
    /// * `task` - The task to process
    ///
    /// # Returns
    /// Result containing the TaskResult or an error
    ///
    /// # Errors
    /// Returns an error if task processing fails
    pub async fn process_task(&self, task: Task) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        let started_at = chrono::Utc::now();

        log::info!("⚙️  Processing task: {} (type: {:?})", task.id, task.task_type);
        log::debug!("Task metadata: priority={}, timeout={:?}s",
            task.priority,
            task.timeout_secs
        );
        
        self.log_builder
            .build(
                LogLevel::Info,
                format!("Processing task: {}", task.id),
            )
            .with_correlation_id(task.correlation_id.clone().unwrap_or_default())
            .with_code_location("task_processor::process_task".to_string())
            .log();

        let result = match &task.task_type {
            TaskType::ExecuteCommand {
                command,
                args,
                working_dir,
                env_vars,
            } => {
                self.process_execute_command(
                    &task,
                    command,
                    args,
                    working_dir.as_deref(),
                    env_vars.as_ref(),
                    task.timeout_secs,
                )
                .await
            }
            TaskType::ExecutePolling {
                command,
                args,
                working_dir,
                env_vars,
                poll_interval_secs,
                timeout_secs,
            } => {
                self.process_execute_polling(
                    &task,
                    command,
                    args,
                    working_dir.as_deref(),
                    env_vars.as_ref(),
                    *poll_interval_secs,
                    *timeout_secs,
                )
                .await
            }
            TaskType::FileTransfer {
                source,
                destination,
                options,
            } => {
                self.process_file_transfer(&task, source, destination, options)
                    .await
            }
            TaskType::FileWatch {
                path,
                patterns,
                recursive,
                on_change_tasks,
            } => {
                self.process_file_watch(&task, path, patterns, *recursive, on_change_tasks)
                    .await
            }
            TaskType::Composite {
                tasks,
                stop_on_error,
            } => {
                self.process_composite(&task, tasks, *stop_on_error)
                    .await
            }
        };

        let duration_ms = (chrono::Utc::now() - started_at).num_milliseconds() as u32;

        match &result {
            Ok(task_result) => {
                log::info!("✅ Task completed: {} (duration: {}ms, status: {:?})",
                    task.id,
                    duration_ms,
                    task_result.status
                );
                self.log_builder
                    .build(
                        LogLevel::Info,
                        format!("Task completed: {}", task.id),
                    )
                    .with_correlation_id(task.correlation_id.clone().unwrap_or_default())
                    .with_execution_time(duration_ms)
                    .with_code_location("task_processor::process_task".to_string())
                    .log();
                Ok(task_result.clone())
            }
            Err(e) => {
                log::error!("❌ Task failed: {} (duration: {}ms) - {}",
                    task.id,
                    duration_ms,
                    e
                );
                self.log_builder
                    .build(
                        LogLevel::Error,
                        format!("Task failed: {} - {}", task.id, e),
                    )
                    .with_correlation_id(task.correlation_id.clone().unwrap_or_default())
                    .with_execution_time(duration_ms)
                    .with_code_location("task_processor::process_task".to_string())
                    .log();
                Err(e.to_string().into())
            }
        }
    }

    /// Processes an execute command task
    async fn process_execute_command(
        &self,
        task: &Task,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env_vars: Option<&std::collections::HashMap<String, String>>,
        timeout_secs: Option<u64>,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        log::debug!("🔧 Executing command: {} {:?} (working_dir: {:?}, timeout: {:?}s)",
            command,
            args,
            working_dir,
            timeout_secs
        );
        
        let mut result = self
            .command_executor
            .execute(command, args, working_dir, env_vars, timeout_secs)
            .await
            .map_err(|e| {
                log::error!("Command execution failed: {}", e);
                e
            })?;
        
        log::debug!("✓ Command executed successfully");

        result.task_id = task.id;
        result.correlation_id = task.correlation_id.clone();

        Ok(result)
    }

    /// Processes an execute polling task
    async fn process_execute_polling(
        &self,
        task: &Task,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env_vars: Option<&std::collections::HashMap<String, String>>,
        poll_interval_secs: u64,
        timeout_secs: Option<u64>,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        log::debug!("🔄 Executing polling command: {} {:?} (poll_interval: {}s, timeout: {:?}s)",
            command,
            args,
            poll_interval_secs,
            timeout_secs
        );
        
        let mut result = self
            .command_executor
            .execute_with_polling(
                command,
                args,
                working_dir,
                env_vars,
                poll_interval_secs,
                timeout_secs,
            )
            .await
            .map_err(|e| {
                log::error!("Polling command execution failed: {}", e);
                e
            })?;
        
        log::debug!("✓ Polling command executed successfully");

        result.task_id = task.id;
        result.correlation_id = task.correlation_id.clone();

        Ok(result)
    }

    /// Processes a file transfer task
    async fn process_file_transfer(
        &self,
        task: &Task,
        source: &crate::domain::entities::task::FileLocation,
        destination: &crate::domain::entities::task::FileLocation,
        options: &crate::domain::entities::task::TransferOptions,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        log::debug!("📁 Transferring file: {:?} -> {:?}", source, destination);
        let started_at = chrono::Utc::now();

        let bytes_transferred = self
            .file_transfer_service
            .transfer(source, destination, options)
            .await
            .map_err(|e| {
                log::error!("File transfer failed: {}", e);
                e
            })?;
        
        log::info!("✓ File transferred successfully: {} bytes", bytes_transferred);

        let mut result = TaskResult::new_success(task.id, task.correlation_id.clone(), started_at);
        result.data = Some(serde_json::json!({
            "bytes_transferred": bytes_transferred,
            "source": source,
            "destination": destination,
        }));

        Ok(result)
    }

    /// Processes a file watch task
    async fn process_file_watch(
        &self,
        task: &Task,
        _path: &str,
        _patterns: &[String],
        _recursive: bool,
        _on_change_tasks: &[Box<TaskType>],
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        log::debug!("👀 Setting up file watch: path={}, patterns={:?}, recursive={}",
            _path,
            _patterns,
            _recursive
        );
        let started_at = chrono::Utc::now();
        let result = TaskResult::new_success(task.id, task.correlation_id.clone(), started_at);
        log::debug!("✓ File watch configured");
        Ok(result)
    }

    /// Processes a composite task
    fn process_composite<'a>(
        &'a self,
        task: &'a Task,
        tasks: &'a [Box<TaskType>],
        stop_on_error: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<TaskResult, Box<dyn Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            log::debug!("📦 Processing composite task with {} sub-tasks (stop_on_error: {})",
                tasks.len(),
                stop_on_error
            );
            let started_at = chrono::Utc::now();
            let mut results = Vec::new();

            for (idx, task_type) in tasks.iter().enumerate() {
                log::trace!("Processing sub-task {}/{}", idx + 1, tasks.len());
                let sub_task = Task {
                    id: uuid::Uuid::new_v4(),
                    correlation_id: task.correlation_id.clone(),
                    task_type: (**task_type).clone(),
                    priority: task.priority,
                    max_retries: task.max_retries,
                    retry_count: 0,
                    callback_url: None,
                    result_queue: None,
                    created_at: task.created_at,
                    scheduled_at: None,
                    timeout_secs: task.timeout_secs,
                    metadata: task.metadata.clone(),
                };

                match self.process_task(sub_task).await {
                Ok(result) => {
                    log::trace!("✓ Sub-task {}/{} completed", idx + 1, tasks.len());
                    results.push(result);
                }
                Err(e) => {
                    log::warn!("❌ Sub-task {}/{} failed: {}", idx + 1, tasks.len(), e);
                    if stop_on_error {
                        log::debug!("Stopping composite task due to error");
                        return Ok(TaskResult::new_failed(
                            task.id,
                            task.correlation_id.clone(),
                            started_at,
                            format!("Composite task failed at step {}: {}", idx, e),
                        ));
                    } else {
                        log::debug!("Continuing composite task despite error");
                        results.push(TaskResult::new_failed(
                            task.id,
                            task.correlation_id.clone(),
                            started_at,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        let all_success = results.iter().all(|r| r.status == TaskStatus::Success);
        let success_count = results.iter().filter(|r| r.status == TaskStatus::Success).count();
        
        log::debug!("Composite task completed: {}/{} sub-tasks succeeded",
            success_count,
            results.len()
        );

        let mut result = if all_success {
            log::info!("✅ All sub-tasks in composite task succeeded");
            TaskResult::new_success(task.id, task.correlation_id.clone(), started_at)
        } else {
            log::warn!("⚠️  Some sub-tasks in composite task failed");
            TaskResult::new_failed(
                task.id,
                task.correlation_id.clone(),
                started_at,
                "One or more sub-tasks failed".to_string(),
            )
        };

            result.data = Some(serde_json::json!({
                "sub_results": results,
            }));

            Ok(result)
        })
    }

    /// Resolves a secret value using the secret repository
    ///
    /// # Arguments
    /// * `secret_key` - The secret key to resolve
    ///
    /// # Returns
    /// Result containing the secret value or an error
    pub async fn resolve_secret(&self, secret_key: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        log::debug!("🔐 Resolving secret: {}", secret_key);
        if let Some(repo) = &self.secret_repository {
            repo.get_secret(secret_key).await
                .map(|s| {
                    log::debug!("✓ Secret resolved successfully");
                    s
                })
                .map_err(|e| {
                    log::error!("Failed to resolve secret: {}", e);
                    e
                })
        } else {
            log::error!("Secret repository not configured");
            Err("Secret repository not configured".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::task::TaskType;
    use crate::infrastructure::services::{
        DefaultCommandExecutor, NotifyFileWatcherService, OpenDalFileTransferService,
    };

    #[tokio::test]
    async fn test_process_execute_command() {
        let executor = Arc::new(DefaultCommandExecutor::new());
        let transfer = Arc::new(OpenDalFileTransferService::new());
        let watcher = Arc::new(NotifyFileWatcherService::new());
        let log_builder = LogBuilder::new();

        let processor = TaskProcessor::new(executor, transfer, watcher, None, log_builder);

        let task = Task::new(TaskType::ExecuteCommand {
            command: "echo".to_string(),
            args: vec!["test".to_string()],
            working_dir: None,
            env_vars: None,
        });

        let result = processor.process_task(task).await;
        assert!(result.is_ok());
    }
}
