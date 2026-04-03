use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, specta::Type)]
#[serde(default)]
pub struct RepoConfig {
    pub name: String,
    pub repo_path: String,
    pub remote: String,
    pub branch: String,
    pub interval_secs: u64,
    pub sync_new_files: bool,
    pub skip_hooks: bool,
    pub conflict_branch: bool,
    pub commit_message: String,
    pub sync_on_start: bool,
    pub debounce_ms: u64,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            repo_path: String::new(),
            remote: "origin".to_string(),
            branch: String::new(),
            interval_secs: 60,
            sync_new_files: true,
            skip_hooks: false,
            conflict_branch: true,
            commit_message: String::new(),
            sync_on_start: true,
            debounce_ms: 500,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, specta::Type)]
#[serde(default)]
pub struct GlobalSettings {
    pub remote: String,
    pub interval_secs: u64,
    pub commit_message: String,
    pub sync_new_files: bool,
    pub skip_hooks: bool,
    pub conflict_branch: bool,
    pub sync_on_start: bool,
    pub debounce_ms: u64,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            interval_secs: 60,
            commit_message: String::new(),
            sync_new_files: true,
            skip_hooks: false,
            conflict_branch: true,
            sync_on_start: true,
            debounce_ms: 500,
        }
    }
}

impl RepoConfig {
    /// Display name: explicit name if set, otherwise the last path component.
    pub fn display_name(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        std::path::Path::new(&self.repo_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.repo_path)
            .to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, specta::Type)]
pub struct DesktopConfig {
    #[serde(default)]
    pub global: GlobalSettings,
    pub repositories: Vec<RepoConfig>,
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
