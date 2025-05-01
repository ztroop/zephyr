use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandConfig {
    pub name: String,
    pub command: String,
    pub interval_minutes: f64,
    pub max_runtime_minutes: Option<u32>,
    pub enabled: bool,
    pub working_dir: Option<PathBuf>,
    pub environment: Option<Vec<(String, String)>>,
    pub immediate: bool,
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
