use crate::domain::entities::TaskResult;
use crate::domain::services::CommandExecutor;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::error::Error;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

/// Default command executor implementation
/// Executes system commands and captures their output
pub struct DefaultCommandExecutor;

impl DefaultCommandExecutor {
    /// Creates a new DefaultCommandExecutor
    ///
    /// # Returns
    /// A new DefaultCommandExecutor instance
    pub fn new() -> Self {
        Self
    }

    /// Builds a command with the given parameters
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `args` - Arguments to pass to the command
    /// * `working_dir` - Optional working directory
    /// * `env_vars` - Optional environment variables
    ///
    /// # Returns
    /// A configured Command instance
    fn build_command(
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env_vars: Option<&HashMap<String, String>>,
    ) -> Command {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        if let Some(env) = env_vars {
            cmd.envs(env);
        }

        cmd
    }

    /// Reads remaining output from a completed child process
    ///
    /// # Arguments
    /// * `child` - The child process
    ///
    /// # Returns
    /// Result containing (stdout, stderr) or an error
    async fn read_process_output(
        child: &mut tokio::process::Child,
    ) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
        let mut stdout_output = String::new();
        let mut stderr_output = String::new();

        if let Some(stdout) = child.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            reader.read_to_string(&mut stdout_output).await?;
        }

        if let Some(stderr) = child.stderr.as_mut() {
            let mut reader = BufReader::new(stderr);
            reader.read_to_string(&mut stderr_output).await?;
        }

        Ok((stdout_output, stderr_output))
    }

    /// Reads all lines from a stream into a vector
    ///
    /// # Arguments
    /// * `reader` - The buffered reader
    ///
    /// # Returns
    /// Vector of lines read from the stream
    async fn read_lines_from_stream(
        mut reader: tokio::io::Lines<BufReader<impl tokio::io::AsyncRead + Unpin>>,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        while let Ok(Some(line)) = reader.next_line().await {
            lines.push(line);
        }
        lines
    }

    /// Captures output from a running process
    ///
    /// # Arguments
    /// * `child` - The child process
    ///
    /// # Returns
    /// Result containing (stdout, stderr, exit_code) or an error
    async fn capture_output(
        mut child: tokio::process::Child,
    ) -> Result<(String, String, i32), Box<dyn Error + Send + Sync>> {
        log::trace!("Capturing output from child process");

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        let stdout_reader = BufReader::new(stdout).lines();
        let stderr_reader = BufReader::new(stderr).lines();

        // Read stdout and stderr concurrently using tokio::join!
        let (stdout_future, stderr_future, status_future) = tokio::join!(
            Self::read_lines_from_stream(stdout_reader),
            Self::read_lines_from_stream(stderr_reader),
            child.wait()
        );

        let status = status_future?;
        let exit_code = status.code().unwrap_or(-1);

        let stdout_output = stdout_future.join("\n");
        let stderr_output = stderr_future.join("\n");

        log::trace!(
            "Output captured: exit_code={}, stdout_lines={}, stderr_lines={}",
            exit_code,
            stdout_future.len(),
            stderr_future.len()
        );

        Ok((stdout_output, stderr_output, exit_code))
    }

    /// Checks if the command execution has timed out
    ///
    /// # Arguments
    /// * `started_at` - When the command started
    /// * `timeout_secs` - Optional timeout duration in seconds
    ///
    /// # Returns
    /// true if timed out, false otherwise
    fn is_timed_out(started_at: chrono::DateTime<Utc>, timeout_secs: Option<u64>) -> bool {
        if let Some(timeout_duration) = timeout_secs {
            let elapsed = (Utc::now() - started_at).num_seconds() as u64;
            elapsed >= timeout_duration
        } else {
            false
        }
    }

    /// Builds a TaskResult from process completion
    ///
    /// # Arguments
    /// * `task_id` - The task ID
    /// * `started_at` - When the task started
    /// * `exit_code` - The process exit code
    /// * `stdout` - Standard output
    /// * `stderr` - Standard error
    ///
    /// # Returns
    /// A TaskResult with the process outcome
    fn build_result_from_exit(
        task_id: Uuid,
        started_at: chrono::DateTime<Utc>,
        exit_code: i32,
        stdout: String,
        stderr: String,
    ) -> TaskResult {
        log::debug!("Building result: exit_code={}, stdout_len={}, stderr_len={}",
            exit_code, stdout.len(), stderr.len()
        );

        let result = if exit_code == 0 {
            TaskResult::new_success(task_id, None, started_at)
        } else {
            TaskResult::new_failed(
                task_id,
                None,
                started_at,
                format!("Command failed with exit code {}", exit_code),
            )
        };

        result
            .with_stdout(stdout)
            .with_stderr(stderr)
            .with_exit_code(exit_code)
    }

    /// Handles a completed process in polling mode
    ///
    /// # Arguments
    /// * `child` - The child process
    /// * `task_id` - The task ID
    /// * `started_at` - When the task started
    /// * `status` - The exit status
    ///
    /// # Returns
    /// Result containing the TaskResult
    async fn handle_process_completion(
        child: &mut tokio::process::Child,
        task_id: Uuid,
        started_at: chrono::DateTime<Utc>,
        status: std::process::ExitStatus,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        log::debug!("Process completed with status: {:?}", status);

        let (stdout_output, stderr_output) = Self::read_process_output(child).await?;
        let exit_code = status.code().unwrap_or(-1);

        Ok(Self::build_result_from_exit(
            task_id,
            started_at,
            exit_code,
            stdout_output,
            stderr_output,
        ))
    }

    /// Handles timeout in polling mode
    ///
    /// # Arguments
    /// * `child` - The child process to kill
    /// * `task_id` - The task ID
    /// * `started_at` - When the task started
    /// * `timeout_duration` - The timeout duration that was exceeded
    ///
    /// # Returns
    /// Result containing the failed TaskResult
    async fn handle_timeout(
        child: &mut tokio::process::Child,
        task_id: Uuid,
        started_at: chrono::DateTime<Utc>,
        timeout_duration: u64,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        log::warn!("Command execution timed out after {} seconds", timeout_duration);
        child.kill().await?;
        Ok(TaskResult::new_failed(
            task_id,
            None,
            started_at,
            format!(
                "Command execution timed out after {} seconds",
                timeout_duration
            ),
        ))
    }
}

impl Default for DefaultCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandExecutor for DefaultCommandExecutor {
    async fn execute(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env_vars: Option<&HashMap<String, String>>,
        timeout_secs: Option<u64>,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        let started_at = Utc::now();
        let task_id = Uuid::new_v4();

        let mut cmd = Self::build_command(command, args, working_dir, env_vars);
        let child = cmd.spawn()?;

        let execution_future = Self::capture_output(child);

        let (stdout_output, stderr_output, exit_code) = if let Some(timeout_duration) = timeout_secs
        {
            match timeout(Duration::from_secs(timeout_duration), execution_future).await {
                Ok(result) => result?,
                Err(_) => {
                    return Ok(TaskResult::new_failed(
                        task_id,
                        None,
                        started_at,
                        format!("Command execution timed out after {} seconds", timeout_duration),
                    ));
                }
            }
        } else {
            execution_future.await?
        };

        let mut result = if exit_code == 0 {
            TaskResult::new_success(task_id, None, started_at)
        } else {
            TaskResult::new_failed(
                task_id,
                None,
                started_at,
                format!("Command failed with exit code {}", exit_code),
            )
        };

        result = result
            .with_stdout(stdout_output)
            .with_stderr(stderr_output)
            .with_exit_code(exit_code);

        Ok(result)
    }

    async fn execute_with_polling(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env_vars: Option<&HashMap<String, String>>,
        poll_interval_secs: u64,
        timeout_secs: Option<u64>,
    ) -> Result<TaskResult, Box<dyn Error + Send + Sync>> {
        log::debug!("Starting polling execution: command={}, poll_interval={}s, timeout={:?}s",
            command, poll_interval_secs, timeout_secs
        );

        let started_at = Utc::now();
        let task_id = Uuid::new_v4();

        let mut cmd = Self::build_command(command, args, working_dir, env_vars);
        let mut child = cmd.spawn()?;

        let pid = child.id().ok_or("Failed to get process ID")?;
        log::trace!("Process started with PID: {}", pid);

        let poll_duration = Duration::from_secs(poll_interval_secs);

        loop {
            // Check if process has completed
            match child.try_wait()? {
                Some(status) => {
                    return Self::handle_process_completion(&mut child, task_id, started_at, status).await;
                }
                None => {
                    // Check for timeout
                    if Self::is_timed_out(started_at, timeout_secs) {
                        return Self::handle_timeout(
                            &mut child,
                            task_id,
                            started_at,
                            timeout_secs.unwrap(),
                        ).await;
                    }

                    // Wait before next poll
                    log::trace!("Process still running, waiting {}s before next poll", poll_interval_secs);
                    tokio::time::sleep(poll_duration).await;
                }
            }
        }
    }

    async fn is_process_running(&self, pid: u32) -> Result<bool, Box<dyn Error + Send + Sync>> {
        #[cfg(unix)]
        {
            use std::process::Command as StdCommand;
            let output = StdCommand::new("ps")
                .args(["-p", &pid.to_string()])
                .output()?;
            Ok(output.status.success())
        }

        #[cfg(windows)]
        {
            use std::process::Command as StdCommand;
            let output = StdCommand::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid)])
                .output()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.contains(&pid.to_string()))
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err("Platform not supported for process checking".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
        let executor = DefaultCommandExecutor::new();

        #[cfg(unix)]
        let result = executor
            .execute("echo", &["hello".to_string()], None, None, None)
            .await
            .unwrap();

        #[cfg(windows)]
        let result = executor
            .execute("cmd", &["/C".to_string(), "echo hello".to_string()], None, None, None)
            .await
            .unwrap();

        assert!(result.is_success());
        assert_eq!(result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_execute_with_timeout() {
        let executor = DefaultCommandExecutor::new();

        #[cfg(unix)]
        let result = executor
            .execute("sleep", &["10".to_string()], None, None, Some(1))
            .await
            .unwrap();

        #[cfg(windows)]
        let result = executor
            .execute(
                "timeout",
                &["/T".to_string(), "10".to_string()],
                None,
                None,
                Some(1),
            )
            .await
            .unwrap();

        assert!(result.is_failed());
        assert!(result.error_message.unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn test_execute_with_working_dir() {
        let executor = DefaultCommandExecutor::new();

        #[cfg(unix)]
        let result = executor
            .execute("pwd", &[], Some("/tmp"), None, None)
            .await
            .unwrap();

        #[cfg(windows)]
        let result = executor
            .execute("cd", &[], Some("C:\\Windows"), None, None)
            .await
            .unwrap();

        assert!(result.is_success());
    }
}
