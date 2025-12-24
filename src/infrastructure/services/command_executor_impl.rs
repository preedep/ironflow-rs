use crate::domain::entities::TaskResult;
use crate::domain::services::CommandExecutor;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::error::Error;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
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
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        let stdout_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                stdout_lines.push(line);
            }
            stdout_lines
        });

        let stderr_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                stderr_lines.push(line);
            }
            stderr_lines
        });

        let status = child.wait().await?;
        let stdout_lines = stdout_task.await?;
        let stderr_lines = stderr_task.await?;

        let stdout_output = stdout_lines.join("\n");
        let stderr_output = stderr_lines.join("\n");
        let exit_code = status.code().unwrap_or(-1);

        Ok((stdout_output, stderr_output, exit_code))
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
        let started_at = Utc::now();
        let task_id = Uuid::new_v4();

        let mut cmd = Self::build_command(command, args, working_dir, env_vars);
        let mut child = cmd.spawn()?;

        let _pid = child.id().ok_or("Failed to get process ID")?;
        let poll_duration = Duration::from_secs(poll_interval_secs);

        loop {
            match child.try_wait()? {
                Some(status) => {
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    let mut stdout_output = String::new();
                    let mut stderr_output = String::new();

                    if let Some(stdout) = stdout {
                        let mut reader = BufReader::new(stdout);
                        reader.read_line(&mut stdout_output).await?;
                    }

                    if let Some(stderr) = stderr {
                        let mut reader = BufReader::new(stderr);
                        reader.read_line(&mut stderr_output).await?;
                    }

                    let exit_code = status.code().unwrap_or(-1);

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

                    return Ok(result);
                }
                None => {
                    if let Some(timeout_duration) = timeout_secs {
                        let elapsed = (Utc::now() - started_at).num_seconds() as u64;
                        if elapsed >= timeout_duration {
                            child.kill().await?;
                            return Ok(TaskResult::new_failed(
                                task_id,
                                None,
                                started_at,
                                format!(
                                    "Command execution timed out after {} seconds",
                                    timeout_duration
                                ),
                            ));
                        }
                    }

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
