use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::PathBuf;
use anyhow::{bail, Result};
use serde::{Serialize, Deserialize};
use directories::ProjectDirs;

#[serde_with::skip_serializing_none]
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    limit_commits: Option<bool>,
    commit_count: Option<usize>,
    username: Option<String>,
}

impl Config {
    pub fn new_default() -> Self {
        Self {
            limit_commits: Some(true),
            commit_count: Some(2000),
            username: None,
        }
    }

    pub fn borrow_limit_commits(&self) -> &Option<bool> {
        &self.limit_commits
    }

    pub fn borrow_commit_count(&self) -> &Option<usize> {
        &self.commit_count
    }

    pub fn borrow_username(&self) -> &Option<String> {
        &self.username
    }

    pub fn set_username(&mut self, new_username: String) {
        self.username = Some(new_username);
    }

    pub fn save(&self) -> Result<()> {
        let config_path_buf = get_config_path()?;
        let config_path = config_path_buf.as_path();
        if !config_path.exists() {
            let prefix = match config_path.parent() {
                Some(p) => p,
                None => bail!("Config path prefix not defined. This should never happen if the library is working."),
            };
            if !prefix.exists() {
                create_dir_all(prefix)?;
            }
        }
        std::fs::write(config_path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }
}

fn get_config_path() -> Result<PathBuf> {
    let pd = match ProjectDirs::from("com", "Oxidized Git", "Oxidized Git") {
        Some(pd) => pd,
        None => bail!("Failed to determine HOME directory on your OS"),
    };
    let config_path = pd.config_dir();
    let mut config_path_buf = config_path.to_path_buf();
    config_path_buf.push(PathBuf::from("config.json"));
    Ok(config_path_buf)
}

fn save_default_config() -> Result<()> {
    let config = Config::new_default();
    config.save()?;
    Ok(())
}

pub fn save_config_from_json(payload: &str) -> Result<()> {
    let config: Config = serde_json::from_str(payload)?;
    config.save()?;
    Ok(())
}

pub fn get_config() -> Result<Config> {
    let config_path_buf = get_config_path()?;
    let config_path = config_path_buf.as_path();
    if !config_path.exists() {
        save_default_config()?;
    }
    let mut data_string = String::new();
    let mut file = File::open(config_path)?;
    file.read_to_string(&mut data_string)?;
    let config: Config = serde_json::from_str(&*data_string)?;
    Ok(config)
}
