use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use indoc::{formatdoc, indoc};
use directories::ProjectDirs;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    limit_commits: bool,
    commit_count: usize,
}

impl Config {
    pub fn get_limit_commits(&self) -> bool {
        self.limit_commits
    }

    pub fn get_commit_count(&self) -> usize {
        self.commit_count
    }
}

pub fn save_default_preferences() -> Result<(), Box<dyn std::error::Error>> {
    let pd = match ProjectDirs::from("com", "Oxidized Git", "Oxidized Git") {
        Some(pd) => pd,
        None => return Err("Failed to determine HOME directory on your OS".into()),
    };
    let config_path = pd.config_dir();
    config_path.to_path_buf().push(PathBuf::from("config.json"));
    if !config_path.exists() {
        let mut file = File::create(config_path)?;
        // TODO: Maybe make a config struct if more options are added in the future?
        let config_str = indoc! {"
            {
                \"limit_commits\": true,
                \"commit_count\": 2000
            }
        "};
        file.write_all(config_str.as_bytes())?;
    }
    Ok(())
}

pub fn save_preferences(payload: &str) -> Result<(), Box<dyn std::error::Error>> {
    let preferences_json: HashMap<String, String> = serde_json::from_str(payload)?;
    let limit_commits = match preferences_json.get("limitCommits") {
        Some(b) => b,
        None => return Err("limitCommits not found in payload from front-end".into()),
    };
    let commit_count = match preferences_json.get("commitCount") {
        Some(c) => c,
        None => return Err("commitCount not found in payload from front-end".into()),
    };
    let pd = match ProjectDirs::from("com", "Oxidized Git", "Oxidized Git") {
        Some(pd) => pd,
        None => return Err("Failed to determine HOME directory on your OS".into()),
    };
    let config_path = pd.config_dir();
    config_path.to_path_buf().push(PathBuf::from("config.json"));
    let mut file = File::create(config_path)?;
    // TODO: Maybe make a config struct if more options are added in the future?
    let config_str = formatdoc! {"
        {{
            \"limit_commits\": {limit_commits},
            \"commit_count\": {commit_count}
        }}
    "};
    file.write_all(config_str.as_bytes())?;
    Ok(())
}

pub fn get_preferences() -> Result<Config, Box<dyn std::error::Error>> {
    let pd = match ProjectDirs::from("com", "Oxidized Git", "Oxidized Git") {
        Some(pd) => pd,
        None => return Err("Failed to determine HOME directory on your OS".into()),
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