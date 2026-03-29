use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DesktopConfig {
    pub repo_path: String,
    pub remote: String,
    pub branch: String,
    pub interval_secs: u64,
    pub sync_new_files: bool,
    pub skip_hooks: bool,
    pub conflict_branch: bool,
    pub commit_message: String,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            remote: "origin".to_string(),
            branch: String::new(),
            interval_secs: 60,
            sync_new_files: true,
            skip_hooks: false,
            conflict_branch: true,
            commit_message: String::new(),
        }
    }
}

fn config_dir() -> PathBuf {
    ProjectDirs::from("com", "Cognito Forms", "Git Sync")
        .expect("Missing home directory")
        .config_dir()
        .into()
}

fn config_path() -> PathBuf {
    config_dir().join("desktop.toml")
}

pub fn load_config() -> DesktopConfig {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &DesktopConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, toml::to_string_pretty(cfg)?)?;
    Ok(())
}
