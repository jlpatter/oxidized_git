use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use anyhow::{bail, Result};
use serde::{Serialize, Deserialize};
use directories::ProjectDirs;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    limit_commits: bool,
    commit_count: usize,
}

impl Config {
    pub fn new_default() -> Self {
        Self {
            limit_commits: true,
            commit_count: 2000,
        }
    }

    pub fn get_limit_commits(&self) -> bool {
        self.limit_commits
    }

    pub fn get_commit_count(&self) -> usize {
        self.commit_count
    }
}

pub fn save_default_preferences() -> Result<()> {
    let pd = match ProjectDirs::from("com", "Oxidized Git", "Oxidized Git") {
        Some(pd) => pd,
        None => bail!("Failed to determine HOME directory on your OS"),
    };
    let config_path = pd.config_dir();
    config_path.to_path_buf().push(PathBuf::from("config.json"));
    if !config_path.exists() {
        let config = Config::new_default();
        serde_json::to_writer(&File::create(config_path)?, &config)?;
    }
    Ok(())
}

pub fn save_preferences(payload: &str) -> Result<()> {
    let config: Config = serde_json::from_str(payload)?;
    let pd = match ProjectDirs::from("com", "Oxidized Git", "Oxidized Git") {
        Some(pd) => pd,
        None => bail!("Failed to determine HOME directory on your OS"),
    };
    let config_path = pd.config_dir();
    config_path.to_path_buf().push(PathBuf::from("config.json"));
    serde_json::to_writer(&File::create(config_path)?, &config)?;
    Ok(())
}

pub fn get_preferences() -> Result<Config> {
    let pd = match ProjectDirs::from("com", "Oxidized Git", "Oxidized Git") {
        Some(pd) => pd,
        None => bail!("Failed to determine HOME directory on your OS"),
    };
    let config_path = pd.config_dir();
    config_path.to_path_buf().push(PathBuf::from("config.json"));
    if !config_path.exists() {
        save_default_preferences()?;
    }
    let mut data_string = String::new();
    let mut file = File::open(config_path)?;
    file.read_to_string(&mut data_string)?;
    let preferences_json: Config = serde_json::from_str(&*data_string)?;
    Ok(preferences_json)
}
