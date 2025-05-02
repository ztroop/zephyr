#![allow(dead_code)]

use crate::config::CommandConfig;
use std::io;
use tokio::process::Command;

/// Represents the output of a command execution
#[derive(Debug)]
pub struct CommandOutput {
    /// The standard output of the command
    pub stdout: Vec<u8>,
    /// The standard error output of the command
    pub stderr: Vec<u8>,
    /// The exit status of the command
    pub status: i32,
}

/// Trait for executing commands with different implementations
#[async_trait::async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Executes a command and returns its output
    ///
    /// # Arguments
    ///
    /// * `command` - The command configuration to execute
    ///
    /// # Returns
    ///
    /// * `Ok(CommandOutput)` - If the command executed successfully
    /// * `Err(io::Error)` - If there was an error executing the command
    async fn execute(&self, command: &CommandConfig) -> io::Result<CommandOutput>;
}

/// Default implementation of CommandExecutor that uses the system shell
pub struct DefaultExecutor;

#[async_trait::async_trait]
impl CommandExecutor for DefaultExecutor {
    async fn execute(&self, command: &CommandConfig) -> io::Result<CommandOutput> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&command.command);

        if let Some(dir) = &command.working_dir {
            cmd.current_dir(dir);
        }

        if let Some(env) = &command.environment {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let output = cmd.output().await?;
        Ok(CommandOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            status: output.status.code().unwrap_or(-1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_command(command: &str) -> CommandConfig {
        CommandConfig {
            name: "test".to_string(),
            command: command.to_string(),
            interval_minutes: Some(1.0),
            cron: None,
            max_runtime_minutes: Some(5),
            enabled: true,
            working_dir: None,
            environment: None,
            immediate: false,
        }
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        let executor = DefaultExecutor;
        let command = create_test_command("echo 'Hello, World!'");

        let output = executor.execute(&command).await.unwrap();
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "Hello, World!"
        );
        assert!(output.stderr.is_empty());
        assert_eq!(output.status, 0);
    }

    #[tokio::test]
    async fn test_execute_with_working_dir() {
        let executor = DefaultExecutor;
        let temp_dir = tempdir().unwrap();
        let command = CommandConfig {
            name: "test".to_string(),
            command: "pwd".to_string(),
            interval_minutes: Some(1.0),
            cron: None,
            max_runtime_minutes: Some(5),
            enabled: true,
            working_dir: Some(temp_dir.path().to_path_buf()),
            environment: None,
            immediate: false,
        };

        let output = executor.execute(&command).await.unwrap();
        let actual_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let expected_path = temp_dir.path().to_string_lossy().to_string();

        // Normalize paths by removing /private prefix if present
        let actual_path = actual_path.replace("/private", "");
        let expected_path = expected_path.replace("/private", "");

        assert_eq!(actual_path, expected_path);
        assert_eq!(output.status, 0);
    }

    #[tokio::test]
    async fn test_execute_with_environment() {
        let executor = DefaultExecutor;
        let command = CommandConfig {
            name: "test".to_string(),
            command: "echo $TEST_VAR".to_string(),
            interval_minutes: Some(1.0),
            cron: None,
            max_runtime_minutes: Some(5),
            enabled: true,
            working_dir: None,
            environment: Some(vec![("TEST_VAR".to_string(), "test_value".to_string())]),
            immediate: false,
        };

        let output = executor.execute(&command).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "test_value");
        assert_eq!(output.status, 0);
    }

    #[tokio::test]
    async fn test_execute_invalid_command() {
        let executor = DefaultExecutor;
        // Use a command that will definitely fail (exit with non-zero status)
        let command = create_test_command("false");

        let output = executor.execute(&command).await.unwrap();
        assert_eq!(output.status, 1); // false command exits with status 1
    }
}
