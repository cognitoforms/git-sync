use crate::error::{Result, SyncError};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Complete configuration for git-sync-rs
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: DefaultConfig,

    #[serde(default)]
    pub repositories: Vec<RepositoryConfig>,
}

/// Default configuration values
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultConfig {
    #[serde(default = "default_sync_interval")]
    pub sync_interval: u64, // seconds

    #[serde(default = "default_sync_new_files")]
    pub sync_new_files: bool,

    #[serde(default)]
    pub skip_hooks: bool,

    #[serde(default = "default_commit_message")]
    pub commit_message: String,

    #[serde(default = "default_remote")]
    pub remote: String,

    /// When true, create a fallback branch on merge conflicts instead of failing
    #[serde(default)]
    pub conflict_branch: bool,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            sync_interval: default_sync_interval(),
            sync_new_files: default_sync_new_files(),
            skip_hooks: false,
            commit_message: default_commit_message(),
            remote: default_remote(),
            conflict_branch: false,
        }
    }
}

/// Repository-specific configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepositoryConfig {
    pub path: PathBuf,

    #[serde(default)]
    pub sync_new_files: Option<bool>,

    #[serde(default)]
    pub skip_hooks: Option<bool>,

    #[serde(default)]
    pub commit_message: Option<String>,

    #[serde(default)]
    pub remote: Option<String>,

    #[serde(default)]
    pub branch: Option<String>,

    #[serde(default)]
    pub watch: bool,

    #[serde(default)]
    pub interval: Option<u64>, // seconds

    /// When true, create a fallback branch on merge conflicts instead of failing
    #[serde(default)]
    pub conflict_branch: Option<bool>,
}

// Default value functions for serde
fn default_sync_interval() -> u64 {
    60
}

fn default_sync_new_files() -> bool {
    true
}

fn default_commit_message() -> String {
    "changes from {hostname} on {timestamp}".to_string()
}

fn default_remote() -> String {
    "origin".to_string()
}

/// Configuration loader that merges multiple sources with correct precedence
pub struct ConfigLoader {
    config_path: Option<PathBuf>,
    cached_config: std::cell::RefCell<Option<Config>>,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            config_path: None,
            cached_config: std::cell::RefCell::new(None),
        }
    }

    pub fn with_config_path(mut self, path: impl AsRef<Path>) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader {
    pub fn load(&self) -> Result<Config> {
        if let Some(cached) = self.cached_config.borrow().as_ref() {
            return Ok(cached.clone());
        }

        let mut config = Config::default();

        if let Some(toml_config) = self.load_toml_config()? {
            debug!("Loaded TOML configuration");
            config = toml_config;
        }

        self.apply_env_vars(&mut config);

        *self.cached_config.borrow_mut() = Some(config.clone());

        Ok(config)
    }

    pub fn load_for_repo(&self, repo_path: &Path) -> Result<RepositoryConfig> {
        let config = self.load()?;

        let repo_config = config
            .repositories
            .into_iter()
            .find(|r| r.path == repo_path)
            .unwrap_or_else(|| RepositoryConfig {
                path: repo_path.to_path_buf(),
                sync_new_files: None,
                skip_hooks: None,
                commit_message: None,
                remote: None,
                branch: None,
                watch: false,
                interval: None,
                conflict_branch: None,
            });

        Ok(repo_config)
    }

    pub fn to_sync_config(
        &self,
        repo_path: &Path,
        cli_new_files: Option<bool>,
        cli_remote: Option<String>,
    ) -> Result<crate::sync::SyncConfig> {
        let config = self.load()?;
        let repo_config = self.load_for_repo(repo_path)?;

        Ok(crate::sync::SyncConfig {
            sync_new_files: cli_new_files
                .or(env::var("GIT_SYNC_NEW_FILES")
                    .ok()
                    .and_then(|v| v.parse().ok()))
                .or(repo_config.sync_new_files)
                .unwrap_or(config.defaults.sync_new_files),

            skip_hooks: repo_config.skip_hooks.unwrap_or(config.defaults.skip_hooks),

            commit_message: repo_config
                .commit_message
                .or(Some(config.defaults.commit_message)),

            remote_name: cli_remote
                .or(env::var("GIT_SYNC_REMOTE").ok())
                .or(repo_config.remote)
                .unwrap_or(config.defaults.remote),

            branch_name: repo_config.branch.clone().unwrap_or_default(),

            conflict_branch: repo_config
                .conflict_branch
                .unwrap_or(config.defaults.conflict_branch),

            target_branch: repo_config.branch,
        })
    }

    fn load_toml_config(&self) -> Result<Option<Config>> {
        let config_path = if let Some(path) = &self.config_path {
            path.clone()
        } else {
            let project_dirs = ProjectDirs::from("", "", "git-sync-rs").ok_or_else(|| {
                SyncError::Other("Could not determine config directory".to_string())
            })?;

            project_dirs.config_dir().join("config.toml")
        };

        if !config_path.exists() {
            debug!("Config file not found at {:?}", config_path);
            return Ok(None);
        }

        info!("Loading config from {:?}", config_path);
        let contents = fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&contents)
            .map_err(|e| SyncError::Other(format!("Failed to parse config: {}", e)))?;

        for repo in &mut config.repositories {
            let expanded = shellexpand::tilde(&repo.path.to_string_lossy()).to_string();
            repo.path = PathBuf::from(expanded);
        }

        Ok(Some(config))
    }

    fn apply_env_vars(&self, config: &mut Config) {
        if let Ok(interval) = env::var("GIT_SYNC_INTERVAL") {
            if let Ok(secs) = interval.parse::<u64>() {
                debug!("Setting sync interval from env: {}s", secs);
                config.defaults.sync_interval = secs;
            }
        }

        if let Ok(new_files) = env::var("GIT_SYNC_NEW_FILES") {
            if let Ok(enabled) = new_files.parse::<bool>() {
                debug!("Setting sync_new_files from env: {}", enabled);
                config.defaults.sync_new_files = enabled;
            }
        }

        if let Ok(remote) = env::var("GIT_SYNC_REMOTE") {
            debug!("Setting remote from env: {}", remote);
            config.defaults.remote = remote;
        }

        if let Ok(msg) = env::var("GIT_SYNC_COMMIT_MESSAGE") {
            debug!("Setting commit message from env");
            config.defaults.commit_message = msg;
        }

        if let Ok(dir) = env::var("GIT_SYNC_DIRECTORY") {
            let expanded = shellexpand::tilde(&dir).to_string();
            let path = PathBuf::from(expanded);
            if !config.repositories.iter().any(|r| r.path == path) {
                debug!("Adding repository from GIT_SYNC_DIRECTORY env: {:?}", path);
                config.repositories.push(RepositoryConfig {
                    path,
                    sync_new_files: None,
                    skip_hooks: None,
                    commit_message: None,
                    remote: None,
                    branch: None,
                    watch: true,
                    interval: None,
                    conflict_branch: None,
                });
            }
        }
    }
}
