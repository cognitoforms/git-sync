use std::sync::{Arc, Mutex};
use std::time::Duration;

use git_sync_rs::{RepositorySynchronizer, SyncConfig, WatchConfig, WatchManager};

use crate::config::DesktopConfig;
use crate::status::{AppStatus, repo_state_label, sync_state_id, sync_state_label};

pub enum BgCmd {
    SyncNow,
    Reconfigure(DesktopConfig),
}

fn build_sync_config(cfg: &DesktopConfig) -> SyncConfig {
    SyncConfig {
        sync_new_files: cfg.sync_new_files,
        skip_hooks: cfg.skip_hooks,
        commit_message: (!cfg.commit_message.is_empty()).then(|| cfg.commit_message.clone()),
        remote_name: cfg.remote.clone(),
        branch_name: cfg.branch.clone(),
        conflict_branch: cfg.conflict_branch,
        ..SyncConfig::default()
    }
}

fn build_watch_config(cfg: &DesktopConfig) -> WatchConfig {
    WatchConfig {
        debounce_ms: 500,
        min_interval_ms: 5_000,
        sync_on_start: true,
        dry_run: false,
        enable_tray: false,
        tray_icon: None,
        periodic_sync_interval_ms: Some(cfg.interval_secs * 1_000),
    }
}

/// Entry point for the background thread — creates a single-threaded tokio
/// runtime and drives the async worker inside a `LocalSet` (required because
/// `git2::Repository` is `!Sync`).
pub fn run_background(
    initial_cfg: DesktopConfig,
    status: Arc<Mutex<AppStatus>>,
    rx: tokio::sync::mpsc::UnboundedReceiver<BgCmd>,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, run_bg_async(initial_cfg, status, rx));
}

async fn run_bg_async(
    mut cfg: DesktopConfig,
    status: Arc<Mutex<AppStatus>>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<BgCmd>,
) {
    'outer: loop {
        if cfg.repo_path.is_empty() {
            {
                let mut s = status.lock().unwrap();
                s.sync_state_label = "No repository configured".to_string();
                s.sync_state_id = "unknown".to_string();
                s.repo_state_label = "—".to_string();
            }
            loop {
                match rx.recv().await {
                    Some(BgCmd::Reconfigure(new_cfg)) => { cfg = new_cfg; continue 'outer; }
                    Some(BgCmd::SyncNow) => {}
                    None => return,
                }
            }
        }

        let wm = WatchManager::new(
            &cfg.repo_path,
            build_sync_config(&cfg),
            build_watch_config(&cfg),
        );

        let status_sync = RepositorySynchronizer::new_with_detected_branch(
            std::path::Path::new(&cfg.repo_path),
            build_sync_config(&cfg),
        );
        match &status_sync {
            Ok(_) => status.lock().unwrap().error = None,
            Err(e) => {
                let mut s = status.lock().unwrap();
                s.error = Some(format!("Cannot open repository: {}", e));
                s.sync_state_label = "Repository error".to_string();
                s.sync_state_id = "error".to_string();
            }
        }

        let status_poll = Arc::clone(&status);
        let poll_handle = tokio::task::spawn_local(async move {
            let sync = status_sync.ok();
            let mut prev_state_id = String::new();
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let mut s = status_poll.lock().unwrap();
                if let Some(ref r) = sync {
                    match r.get_sync_state() {
                        Ok(st) => {
                            let new_id = sync_state_id(&st).to_string();
                            if new_id == "equal"
                                && !prev_state_id.is_empty()
                                && prev_state_id != "equal"
                            {
                                s.last_sync_time = Some(std::time::Instant::now());
                            }
                            prev_state_id = new_id.clone();
                            s.sync_state_label = sync_state_label(&st);
                            s.sync_state_id = new_id;
                        }
                        Err(e) => {
                            s.sync_state_label = format!("Error: {}", e);
                            s.sync_state_id = "error".to_string();
                        }
                    }
                    if let Ok(st) = r.get_repository_state() {
                        s.repo_state_label = repo_state_label(&st).to_string();
                    }
                }
            }
        });

        let mut watch_handle = tokio::task::spawn_local(async move { wm.watch().await });

        tokio::select! {
            result = &mut watch_handle => {
                poll_handle.abort();
                if let Ok(Err(e)) = result {
                    status.lock().unwrap().error = Some(e.to_string());
                }
                continue 'outer;
            }
            cmd = rx.recv() => {
                watch_handle.abort();
                poll_handle.abort();
                match cmd {
                    Some(BgCmd::SyncNow) => continue 'outer,
                    Some(BgCmd::Reconfigure(new_cfg)) => {
                        cfg = new_cfg;
                        continue 'outer;
                    }
                    None => return,
                }
            }
        }
    }
}
