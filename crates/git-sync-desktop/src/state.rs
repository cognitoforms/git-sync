use gpui::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::{DesktopConfig, RepoConfig};
use crate::status::AppStatus;
use crate::worker::BgCmd;

pub struct AppState {
    pub config: DesktopConfig,
    pub status: AppStatus,
    pub worker_tx: UnboundedSender<BgCmd>,
}

impl AppState {
    pub fn new(config: DesktopConfig, worker_tx: UnboundedSender<BgCmd>) -> Self {
        Self {
            config,
            status: AppStatus::default(),
            worker_tx,
        }
    }

    /// Push a new status snapshot and trigger a UI re-render.
    pub fn update_status(&mut self, new_status: AppStatus, cx: &mut Context<Self>) {
        self.status = new_status;
        cx.notify();
    }

    pub fn sync_now(&self, idx: usize) {
        let _ = self.worker_tx.send(BgCmd::SyncNow(idx));
    }

    pub fn save_and_reconfigure(&mut self, new_cfg: DesktopConfig) {
        if let Err(e) = crate::config::save_config(&new_cfg) {
            eprintln!("Failed to save config: {e}");
        }
        let _ = self.worker_tx.send(BgCmd::Reconfigure(new_cfg.clone()));
        self.config = new_cfg;
    }

    /// Save updated settings for a single repo and reconfigure the worker.
    pub fn save_repo(&mut self, idx: usize, repo: RepoConfig) {
        if idx < self.config.repositories.len() {
            self.config.repositories[idx] = repo;
        }
        self.save_and_reconfigure(self.config.clone());
    }

    /// Append a new repo and reconfigure the worker.
    pub fn add_repo(&mut self, repo: RepoConfig) {
        self.config.repositories.push(repo);
        self.save_and_reconfigure(self.config.clone());
    }

    /// Remove a repo by index and reconfigure the worker.
    pub fn delete_repo(&mut self, idx: usize) {
        if idx < self.config.repositories.len() {
            self.config.repositories.remove(idx);
        }
        self.save_and_reconfigure(self.config.clone());
    }
}
