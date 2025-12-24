use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the type of task to be executed by the worker
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskType {
    /// Execute a single command and return immediately
    ExecuteCommand {
        command: String,
        args: Vec<String>,
        working_dir: Option<String>,
        env_vars: Option<std::collections::HashMap<String, String>>,
    },
    /// Execute a command and poll until completion
    ExecutePolling {
        command: String,
        args: Vec<String>,
        working_dir: Option<String>,
        env_vars: Option<std::collections::HashMap<String, String>>,
        poll_interval_secs: u64,
        timeout_secs: Option<u64>,
    },
    /// Watch for file system changes
    FileWatch {
        path: String,
        patterns: Vec<String>,
        recursive: bool,
        on_change_tasks: Vec<Box<TaskType>>,
    },
    /// Transfer files between locations
    FileTransfer {
        source: FileLocation,
        destination: FileLocation,
        options: TransferOptions,
    },
    /// Composite task that executes multiple tasks in sequence
    Composite {
        tasks: Vec<Box<TaskType>>,
        stop_on_error: bool,
    },
}

/// Represents a file location for transfer operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileLocation {
    Local { path: String },
    Ftp { host: String, port: u16, path: String, username: String, password_secret: String },
    Sftp { host: String, port: u16, path: String, username: String, key_secret: String },
    S3 { bucket: String, key: String, region: String, endpoint: Option<String> },
    AzureBlob { account: String, container: String, blob: String },
}

/// Options for file transfer operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransferOptions {
    pub overwrite: bool,
    pub create_dirs: bool,
    pub preserve_metadata: bool,
    pub chunk_size: Option<usize>,
}

impl Default for TransferOptions {
    fn default() -> Self {
        Self {
            overwrite: false,
            create_dirs: true,
            preserve_metadata: true,
            chunk_size: None,
        }
    }
}

/// Represents a task to be executed by the worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for the task
    pub id: Uuid,
    /// Correlation ID for tracking across systems
    pub correlation_id: Option<String>,
    /// Type and parameters of the task
    pub task_type: TaskType,
    /// Priority of the task (higher = more important)
    pub priority: i32,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Current retry attempt
    pub retry_count: u32,
    /// Callback URL to send results to
    pub callback_url: Option<String>,
    /// Queue to send results back to
    pub result_queue: Option<String>,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// When the task should be executed (for scheduled tasks)
    pub scheduled_at: Option<DateTime<Utc>>,
    /// Timeout for task execution in seconds
    pub timeout_secs: Option<u64>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl Task {
    /// Creates a new task with the given type
    ///
    /// # Arguments
    /// * `task_type` - The type and parameters of the task
    ///
    /// # Returns
    /// A new Task instance with default values
    pub fn new(task_type: TaskType) -> Self {
        Self {
            id: Uuid::new_v4(),
            correlation_id: None,
            task_type,
            priority: 0,
            max_retries: 3,
            retry_count: 0,
            callback_url: None,
            result_queue: None,
            created_at: Utc::now(),
            scheduled_at: None,
            timeout_secs: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Sets the correlation ID for the task
    ///
    /// # Arguments
    /// * `correlation_id` - The correlation ID to set
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Sets the callback URL for the task
    ///
    /// # Arguments
    /// * `url` - The callback URL to send results to
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_callback_url(mut self, url: String) -> Self {
        self.callback_url = Some(url);
        self
    }

    /// Sets the result queue for the task
    ///
    /// # Arguments
    /// * `queue` - The queue name to send results to
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_result_queue(mut self, queue: String) -> Self {
        self.result_queue = Some(queue);
        self
    }

    /// Checks if the task can be retried
    ///
    /// # Returns
    /// true if the task has retries remaining, false otherwise
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Increments the retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(TaskType::ExecuteCommand {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            working_dir: None,
            env_vars: None,
        });

        assert_eq!(task.priority, 0);
        assert_eq!(task.max_retries, 3);
        assert_eq!(task.retry_count, 0);
        assert!(task.can_retry());
    }

    #[test]
    fn test_task_retry_logic() {
        let mut task = Task::new(TaskType::ExecuteCommand {
            command: "test".to_string(),
            args: vec![],
            working_dir: None,
            env_vars: None,
        });

        task.max_retries = 2;
        assert!(task.can_retry());

        task.increment_retry();
        assert!(task.can_retry());

        task.increment_retry();
        assert!(!task.can_retry());
    }

    #[test]
    fn test_task_with_correlation_id() {
        let task = Task::new(TaskType::ExecuteCommand {
            command: "test".to_string(),
            args: vec![],
            working_dir: None,
            env_vars: None,
        })
        .with_correlation_id("test-correlation-id".to_string());

        assert_eq!(task.correlation_id, Some("test-correlation-id".to_string()));
    }
}
