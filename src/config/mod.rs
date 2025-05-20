use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
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

        if let Some(parent) = self.state_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    anyhow::anyhow!("Failed to create state directory at {:?}: {}", parent, e)
                })?;
            }
        }

        Ok(())
    }
}

fn default_min_interval_seconds() -> u64 {
    30
}

fn default_state_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Could not find home directory");
    path.push(".local/state/zephyr");
    std::fs::create_dir_all(&path).expect("Failed to create state directory");
    path.push("state.db");
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
    pub enabled: bool,
    pub working_dir: Option<PathBuf>,
    pub environment: Option<Vec<(String, String)>>,
    pub immediate: bool,
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
        for command in &config.commands {
            command.validate()?;
        }

        Ok(config)
    }
}
