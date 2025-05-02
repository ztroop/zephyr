use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
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

        Ok(config.try_deserialize()?)
    }
}
