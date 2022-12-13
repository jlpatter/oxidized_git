use std::fs;
use std::fs::create_dir_all;
use std::path::PathBuf;
use anyhow::{bail, Result};
use serde::{Serialize, Deserialize};
use directories::ProjectDirs;

#[serde_with::skip_serializing_none]
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    limit_commits: Option<bool>,
    commit_count: Option<usize>,
    cred_type: Option<String>,
    https_username: Option<String>,
    public_key_path: Option<PathBuf>,
    private_key_path: Option<PathBuf>,
    uses_passphrase: Option<bool>,
}

impl Config {
    pub fn new_default() -> Self {
        Self {
            limit_commits: Some(true),
            commit_count: Some(2000),
            cred_type: None,
            https_username: None,
            public_key_path: None,
            private_key_path: None,
            uses_passphrase: None,
        }
    }

    pub fn borrow_limit_commits(&self) -> &Option<bool> {
        &self.limit_commits
    }

    pub fn borrow_commit_count(&self) -> &Option<usize> {
        &self.commit_count
    }

    pub fn borrow_cred_type(&self) -> &Option<String> {
        &self.cred_type
    }

    pub fn borrow_https_username(&self) -> &Option<String> {
        &self.https_username
    }

    pub fn borrow_public_key_path(&self) -> &Option<PathBuf> {
        &self.public_key_path
    }

    pub fn borrow_private_key_path(&self) -> &Option<PathBuf> {
        &self.private_key_path
    }

    pub fn borrow_uses_passphrase(&self) -> &Option<bool> {
        &self.uses_passphrase
    }

    pub fn set_cred_type(&mut self, cred_type: String) {
        self.cred_type = Some(cred_type);
    }

    pub fn set_https_username(&mut self, new_username: String) {
        self.https_username = Some(new_username);
    }

    pub fn set_public_key_path(&mut self, public_key_path: PathBuf) {
        self.public_key_path = Some(public_key_path);
    }

    pub fn set_private_key_path(&mut self, private_key_path: PathBuf) {
        self.private_key_path = Some(private_key_path);
    }

    pub fn set_uses_passphrase(&mut self, uses_passphrase: bool) {
        self.uses_passphrase = Some(uses_passphrase);
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
        fs::write(config_path, serde_json::to_string_pretty(&self)?)?;
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
    let data_string = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&*data_string)?;
    Ok(config)
}
