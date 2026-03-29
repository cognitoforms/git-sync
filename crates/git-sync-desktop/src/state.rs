use gpui::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::DesktopConfig;
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

    pub fn sync_now(&self) {
        let _ = self.worker_tx.send(BgCmd::SyncNow);
    }

    pub fn save_and_reconfigure(&mut self, new_cfg: DesktopConfig) {
        if let Err(e) = crate::config::save_config(&new_cfg) {
            eprintln!("Failed to save config: {e}");
        }
        let _ = self.worker_tx.send(BgCmd::Reconfigure(new_cfg.clone()));
        self.config = new_cfg;
    }
}
