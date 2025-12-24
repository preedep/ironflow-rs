use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the status of a task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Timeout,
    Cancelled,
}

/// Represents the result of a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// ID of the task that was executed
    pub task_id: Uuid,
    /// Correlation ID for tracking
    pub correlation_id: Option<String>,
    /// Status of the task execution
    pub status: TaskStatus,
    /// Exit code (for command execution)
    pub exit_code: Option<i32>,
    /// Standard output captured from the task
    pub stdout: Option<String>,
    /// Standard error captured from the task
    pub stderr: Option<String>,
    /// Error message if the task failed
    pub error_message: Option<String>,
    /// When the task started executing
    pub started_at: DateTime<Utc>,
    /// When the task completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration of execution in milliseconds
    pub duration_ms: Option<u64>,
    /// Additional result data
    pub data: Option<serde_json::Value>,
    /// Metadata about the execution
    pub metadata: std::collections::HashMap<String, String>,
}

impl TaskResult {
    /// Creates a new TaskResult with the given status
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task
    /// * `correlation_id` - Optional correlation ID
    /// * `status` - The status of the task
    ///
    /// # Returns
    /// A new TaskResult instance with the specified status
    fn new_with_status(task_id: Uuid, correlation_id: Option<String>, status: TaskStatus) -> Self {
        Self {
            task_id,
            correlation_id,
            status,
            exit_code: None,
            stdout: None,
            stderr: None,
            error_message: None,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            data: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Creates a new TaskResult for a pending task
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task
    /// * `correlation_id` - Optional correlation ID
    ///
    /// # Returns
    /// A new TaskResult instance with pending status
    pub fn new_pending(task_id: Uuid, correlation_id: Option<String>) -> Self {
        Self::new_with_status(task_id, correlation_id, TaskStatus::Pending)
    }

    /// Creates a new TaskResult for a running task
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task
    /// * `correlation_id` - Optional correlation ID
    ///
    /// # Returns
    /// A new TaskResult instance with running status
    pub fn new_running(task_id: Uuid, correlation_id: Option<String>) -> Self {
        Self::new_with_status(task_id, correlation_id, TaskStatus::Running)
    }

    /// Creates a completed TaskResult with duration calculation
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task
    /// * `correlation_id` - Optional correlation ID
    /// * `started_at` - When the task started
    /// * `status` - The final status of the task
    /// * `exit_code` - Optional exit code
    /// * `error_message` - Optional error message
    ///
    /// # Returns
    /// A new TaskResult instance with completion details
    fn new_completed(
        task_id: Uuid,
        correlation_id: Option<String>,
        started_at: DateTime<Utc>,
        status: TaskStatus,
        exit_code: Option<i32>,
        error_message: Option<String>,
    ) -> Self {
        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        Self {
            task_id,
            correlation_id,
            status,
            exit_code,
            stdout: None,
            stderr: None,
            error_message,
            started_at,
            completed_at: Some(completed_at),
            duration_ms: Some(duration_ms),
            data: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Creates a successful TaskResult
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task
    /// * `correlation_id` - Optional correlation ID
    /// * `started_at` - When the task started
    ///
    /// # Returns
    /// A new TaskResult instance with success status
    pub fn new_success(
        task_id: Uuid,
        correlation_id: Option<String>,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self::new_completed(task_id, correlation_id, started_at, TaskStatus::Success, Some(0), None)
    }

    /// Creates a failed TaskResult
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task
    /// * `correlation_id` - Optional correlation ID
    /// * `started_at` - When the task started
    /// * `error_message` - The error message
    ///
    /// # Returns
    /// A new TaskResult instance with failed status
    pub fn new_failed(
        task_id: Uuid,
        correlation_id: Option<String>,
        started_at: DateTime<Utc>,
        error_message: String,
    ) -> Self {
        Self::new_completed(
            task_id,
            correlation_id,
            started_at,
            TaskStatus::Failed,
            None,
            Some(error_message),
        )
    }

    /// Sets the stdout output
    ///
    /// # Arguments
    /// * `stdout` - The standard output to set
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_stdout(mut self, stdout: String) -> Self {
        self.stdout = Some(stdout);
        self
    }

    /// Sets the stderr output
    ///
    /// # Arguments
    /// * `stderr` - The standard error to set
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_stderr(mut self, stderr: String) -> Self {
        self.stderr = Some(stderr);
        self
    }

    /// Sets the exit code
    ///
    /// # Arguments
    /// * `code` - The exit code to set
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = Some(code);
        self
    }

    /// Sets additional data
    ///
    /// # Arguments
    /// * `data` - The data to set
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Checks if the task was successful
    ///
    /// # Returns
    /// true if the task succeeded, false otherwise
    pub fn is_success(&self) -> bool {
        self.status == TaskStatus::Success
    }

    /// Checks if the task failed
    ///
    /// # Returns
    /// true if the task failed, false otherwise
    pub fn is_failed(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Failed | TaskStatus::Timeout | TaskStatus::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_result_success() {
        let task_id = Uuid::new_v4();
        let started_at = Utc::now();
        let result = TaskResult::new_success(task_id, None, started_at);

        assert_eq!(result.task_id, task_id);
        assert_eq!(result.status, TaskStatus::Success);
        assert!(result.is_success());
        assert!(!result.is_failed());
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_task_result_failed() {
        let task_id = Uuid::new_v4();
        let started_at = Utc::now();
        let result = TaskResult::new_failed(
            task_id,
            None,
            started_at,
            "Test error".to_string(),
        );

        assert_eq!(result.task_id, task_id);
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(!result.is_success());
        assert!(result.is_failed());
        assert_eq!(result.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_task_result_with_outputs() {
        let task_id = Uuid::new_v4();
        let result = TaskResult::new_pending(task_id, None)
            .with_stdout("output".to_string())
            .with_stderr("error".to_string())
            .with_exit_code(1);

        assert_eq!(result.stdout, Some("output".to_string()));
        assert_eq!(result.stderr, Some("error".to_string()));
        assert_eq!(result.exit_code, Some(1));
    }
}
