use crate::util::expand_tilde;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_min_interval_seconds")]
    pub min_interval_seconds: u64,
    #[serde(default = "default_state_path")]
    pub state_path: PathBuf,
    #[serde(default = "default_max_immediate_executions")]
    pub max_immediate_executions: usize,
}

impl GeneralConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.min_interval_seconds < 1 {
            return Err(anyhow::anyhow!(
                "min_interval_seconds must be at least 1 second"
            ));
        }

        if self.min_interval_seconds > 3600 {
            return Err(anyhow::anyhow!(
                "min_interval_seconds cannot be greater than 3600 seconds (1 hour)"
            ));
        }

        if self.max_immediate_executions < 1 {
            return Err(anyhow::anyhow!(
                "max_immediate_executions must be at least 1"
            ));
        }

        if self.max_immediate_executions > 100 {
            return Err(anyhow::anyhow!(
                "max_immediate_executions cannot be greater than 100"
            ));
        }

        let expanded_state_path = expand_tilde(&self.state_path);
        if let Some(parent) = expanded_state_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    anyhow::anyhow!("Failed to create state directory at {:?}: {}", parent, e)
                })?;
            }
        }

        Ok(())
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            min_interval_seconds: default_min_interval_seconds(),
            state_path: default_state_path(),
            max_immediate_executions: default_max_immediate_executions(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_min_interval_seconds() -> u64 {
    30
}

fn default_state_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    path.push(".local/state/zephyr/state.db");
    path
}

fn default_max_immediate_executions() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub interval_minutes: Option<f64>,
    #[serde(default)]
    pub cron: Option<String>,
    pub max_runtime_minutes: Option<u32>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub working_dir: Option<PathBuf>,
    pub environment: Option<Vec<(String, String)>>,
    #[serde(default)]
    pub immediate: bool,
}

fn default_enabled() -> bool {
    true
}

impl CommandConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.interval_minutes.is_none() && self.cron.is_none() {
            return Err(anyhow::anyhow!(
                "Command '{}' must specify either interval_minutes or cron",
                self.name
            ));
        }
        if self.interval_minutes.is_some() && self.cron.is_some() {
            return Err(anyhow::anyhow!(
                "Command '{}' cannot specify both interval_minutes and cron",
                self.name
            ));
        }
        if let Some(interval) = self.interval_minutes {
            if interval <= 0.0 {
                return Err(anyhow::anyhow!(
                    "Command '{}' interval_minutes must be positive, got {}",
                    self.name,
                    interval
                ));
            }
        }
        if let Some(max) = self.max_runtime_minutes {
            if max == 0 {
                return Err(anyhow::anyhow!(
                    "Command '{}' max_runtime_minutes must be at least 1",
                    self.name
                ));
            }
        }
        if let Some(cron) = &self.cron {
            cron::Schedule::from_str(cron).map_err(|e| {
                anyhow::anyhow!("Invalid cron expression for command '{}': {}", self.name, e)
            })?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    pub commands: Vec<CommandConfig>,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::from(path))
            .build()?;

        let config: Config = config.try_deserialize()?;
        config.general.validate()?;
        let mut seen = std::collections::HashSet::new();
        for cmd in &config.commands {
            if !seen.insert(cmd.name.as_str()) {
                return Err(anyhow::anyhow!(
                    "Duplicate command name '{}' - command names must be unique",
                    cmd.name
                ));
            }
        }
        for command in &config.commands {
            command.validate()?;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_temp_config(content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("scheduler.toml");
        std::fs::write(&config_path, content).unwrap();
        dir
    }

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
[general]
log_level = "info"
min_interval_seconds = 60
state_path = "/tmp/zephyr/state.db"
max_immediate_executions = 5

[[commands]]
name = "test_cmd"
command = "echo hello"
interval_minutes = 5.0
enabled = true
immediate = false
"#;
        let dir = create_temp_config(config_content);
        let config_path = dir.path().join("scheduler.toml");
        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.general.min_interval_seconds, 60);
        assert_eq!(config.commands.len(), 1);
        assert_eq!(config.commands[0].name, "test_cmd");
    }

    #[test]
    fn test_config_validation_interval_and_cron_mutually_exclusive() {
        let config_content = r#"
[general]
log_level = "info"
state_path = "/tmp/zephyr/state.db"

[[commands]]
name = "bad_cmd"
command = "echo test"
interval_minutes = 5.0
cron = "0 0 * * * *"
enabled = true
immediate = false
"#;
        let dir = create_temp_config(config_content);
        let config_path = dir.path().join("scheduler.toml");
        let result = Config::load(&config_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot specify both"));
    }

    #[test]
    fn test_config_validation_requires_interval_or_cron() {
        let config_content = r#"
[general]
log_level = "info"
state_path = "/tmp/zephyr/state.db"

[[commands]]
name = "bad_cmd"
command = "echo test"
enabled = true
immediate = false
"#;
        let dir = create_temp_config(config_content);
        let config_path = dir.path().join("scheduler.toml");
        let result = Config::load(&config_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must specify either"));
    }

    #[test]
    fn test_config_valid_cron() {
        let config_content = r#"
[general]
log_level = "info"
state_path = "/tmp/zephyr/state.db"

[[commands]]
name = "cron_cmd"
command = "echo test"
cron = "0 0 * * * *"
enabled = true
immediate = false
"#;
        let dir = create_temp_config(config_content);
        let config_path = dir.path().join("scheduler.toml");
        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.commands[0].cron.as_deref(), Some("0 0 * * * *"));
    }

    #[test]
    fn test_config_without_general_uses_defaults() {
        let config_content = r#"
[[commands]]
name = "minimal_cmd"
command = "echo test"
interval_minutes = 5.0
"#;
        let dir = create_temp_config(config_content);
        let config_path = dir.path().join("scheduler.toml");
        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.general.log_level, "info");
        assert_eq!(config.general.min_interval_seconds, 30);
        assert_eq!(config.general.max_immediate_executions, 10);
        assert_eq!(config.commands.len(), 1);
        assert_eq!(config.commands[0].name, "minimal_cmd");
    }

    #[test]
    fn test_config_validation_duplicate_command_names() {
        let config_content = r#"
[general]
log_level = "info"
state_path = "/tmp/zephyr/state.db"

[[commands]]
name = "duplicate_cmd"
command = "echo first"
interval_minutes = 5.0

[[commands]]
name = "duplicate_cmd"
command = "echo second"
interval_minutes = 10.0
"#;
        let dir = create_temp_config(config_content);
        let config_path = dir.path().join("scheduler.toml");
        let result = Config::load(&config_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate command name"));
    }
}
