use crate::domain::entities::TaskResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;

/// Service trait for executing commands
/// This abstraction allows different command execution strategies
/// (single execution, polling, etc.) to be implemented
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Executes a command and returns immediately with the result
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `args` - Arguments to pass to the command
    /// * `working_dir` - Optional working directory
    /// * `env_vars` - Optional environment variables
    /// * `timeout_secs` - Optional timeout in seconds
    ///
    /// # Returns
    /// Result containing the TaskResult with stdout, stderr, and exit code
    ///
    /// # Errors
    /// Returns an error if the command execution fails
    async fn execute(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env_vars: Option<&HashMap<String, String>>,
        timeout_secs: Option<u64>,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>>;

    /// Executes a command and polls until completion
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `args` - Arguments to pass to the command
    /// * `working_dir` - Optional working directory
    /// * `env_vars` - Optional environment variables
    /// * `poll_interval_secs` - Interval between polls in seconds
    /// * `timeout_secs` - Optional timeout in seconds
    ///
    /// # Returns
    /// Result containing the TaskResult after the process completes
    ///
    /// # Errors
    /// Returns an error if the command execution or polling fails
    async fn execute_with_polling(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env_vars: Option<&HashMap<String, String>>,
        poll_interval_secs: u64,
        timeout_secs: Option<u64>,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>>;

    /// Checks if a process is still running
    ///
    /// # Arguments
    /// * `pid` - The process ID to check
    ///
    /// # Returns
    /// Result containing true if the process is running, false otherwise
    ///
    /// # Errors
    /// Returns an error if the check fails
    async fn is_process_running(&self, pid: u32) -> Result<bool, Box<dyn Error + Send + Sync>>;
}
