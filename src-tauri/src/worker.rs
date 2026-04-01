use std::sync::Arc;
use std::time::Duration;

use git_sync_lib::{SyncConfig, WatchConfig, WatchManager};
use tokio::sync::watch;
use tokio::task::AbortHandle;
use tracing::Instrument as _;

use crate::config::{DesktopConfig, RepoConfig};
use crate::status::{
    repo_state_label, sync_state_id, sync_state_label, AppStatus, RepoStatus, SyncErrorPayload,
};

pub enum ConflictResolutionStrategy {
    KeepMine,
    AcceptRemote,
    AbandonConflictBranch,
}

pub enum BgCmd {
    SyncNow(usize),
    Reconfigure(DesktopConfig),
    ResolveConflict {
        index: usize,
        strategy: ConflictResolutionStrategy,
    },
}

pub fn build_sync_config_pub(cfg: &RepoConfig) -> SyncConfig {
    build_sync_config(cfg)
}

fn build_sync_config(cfg: &RepoConfig) -> SyncConfig {
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

fn build_watch_config(cfg: &RepoConfig) -> WatchConfig {
    WatchConfig {
        debounce_ms: cfg.debounce_ms,
        min_interval_ms: 5_000,
        sync_on_start: cfg.sync_on_start,
        dry_run: false,
        periodic_sync_interval_ms: Some(cfg.interval_secs * 1_000),
    }
}

/// Entry point for the background thread — creates a single-threaded tokio
/// runtime and drives the async worker inside a `LocalSet`.
pub fn run_background(
    initial_cfg: DesktopConfig,
    status_tx: Arc<watch::Sender<AppStatus>>,
    rx: tokio::sync::mpsc::UnboundedReceiver<BgCmd>,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, run_bg_async(initial_cfg, status_tx, rx));
}

async fn run_bg_async(
    mut cfg: DesktopConfig,
    status_tx: Arc<watch::Sender<AppStatus>>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<BgCmd>,
) {
    'outer: loop {
        // Initialise status slots to match current config length.
        status_tx.send_if_modified(|s| {
            s.repos = cfg
                .repositories
                .iter()
                .map(|r| RepoStatus::new_loading(r.repo_path.clone()))
                .collect();
            true
        });

        let mut task_handles: Vec<AbortHandle> = Vec::new();
        let mut join_set: tokio::task::JoinSet<usize> = tokio::task::JoinSet::new();

        for (idx, repo_cfg) in cfg.repositories.iter().enumerate() {
            let handle = spawn_repo_task(idx, repo_cfg, Arc::clone(&status_tx), &mut join_set);
            task_handles.push(handle);
        }

        loop {
            tokio::select! {
                Some(result) = join_set.join_next() => {
                    // A task exited (error or EOF). Respawn it after a brief delay.
                    let idx = match result {
                        Ok(i) => i,
                        Err(_) => continue, // cancelled — ignore
                    };
                    if let Some(repo_cfg) = cfg.repositories.get(idx) {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        let handle = spawn_repo_task(idx, repo_cfg, Arc::clone(&status_tx), &mut join_set);
                        if idx < task_handles.len() {
                            task_handles[idx] = handle;
                        }
                    }
                }
                cmd = rx.recv() => {
                    match cmd {
                        Some(BgCmd::Reconfigure(new_cfg)) => {
                            for h in &task_handles { h.abort(); }
                            cfg = new_cfg;
                            continue 'outer;
                        }
                        Some(BgCmd::SyncNow(idx)) => {
                            if let Some(handle) = task_handles.get(idx) {
                                handle.abort();
                            }
                            if let Some(repo_cfg) = cfg.repositories.get(idx) {
                                tokio::task::yield_now().await;
                                let handle = spawn_repo_task(idx, repo_cfg, Arc::clone(&status_tx), &mut join_set);
                                if idx < task_handles.len() {
                                    task_handles[idx] = handle;
                                }
                            }
                        }
                        Some(BgCmd::ResolveConflict { index: idx, strategy }) => {
                            if let Some(handle) = task_handles.get(idx) {
                                handle.abort();
                            }
                            if let Some(repo_cfg) = cfg.repositories.get(idx).cloned() {
                                tokio::task::yield_now().await;
                                let status_tx_clone = Arc::clone(&status_tx);
                                let handle = join_set.spawn_local(async move {
                                    run_resolve(idx, &repo_cfg, strategy, status_tx_clone).await;
                                    idx
                                });
                                if idx < task_handles.len() {
                                    task_handles[idx] = handle;
                                }
                            }
                        }
                        None => return,
                    }
                }
            }
        }
    }
}

/// Spawn an async task that manages a single repository. Returns the AbortHandle.
fn spawn_repo_task(
    idx: usize,
    repo_cfg: &RepoConfig,
    status_tx: Arc<watch::Sender<AppStatus>>,
    join_set: &mut tokio::task::JoinSet<usize>,
) -> AbortHandle {
    let repo_cfg = repo_cfg.clone();

    join_set.spawn_local(async move {
        run_repo(idx, &repo_cfg, status_tx).await;
        idx
    })
}

async fn run_resolve(
    idx: usize,
    cfg: &RepoConfig,
    strategy: ConflictResolutionStrategy,
    status_tx: Arc<watch::Sender<AppStatus>>,
) {
    use git_sync_lib::RepositorySynchronizer;

    status_tx.send_if_modified(|s| {
        if let Some(rs) = s.repos.get_mut(idx) {
            rs.sync_state_label = "Resolving…".to_string();
            rs.sync_state_id = "syncing".to_string();
            rs.is_syncing = true;
            rs.error = None;
            true
        } else {
            false
        }
    });

    let result = RepositorySynchronizer::new_with_detected_branch(&cfg.repo_path, build_sync_config(cfg))
        .and_then(|syncer| match strategy {
            ConflictResolutionStrategy::KeepMine => syncer.resolve_keep_mine(),
            ConflictResolutionStrategy::AcceptRemote => syncer.resolve_accept_remote(),
            ConflictResolutionStrategy::AbandonConflictBranch => syncer.abandon_conflict_branch(),
        });

    status_tx.send_if_modified(|s| {
        if let Some(rs) = s.repos.get_mut(idx) {
            rs.is_syncing = false;
            match result {
                Ok(()) => {
                    rs.error = None;
                    rs.sync_state_label = "Resolved".to_string();
                    rs.sync_state_id = "ok".to_string();
                }
                Err(ref e) => {
                    rs.error = Some(SyncErrorPayload::from(
                        &git_sync_lib::SyncErrorSummary::from(e),
                    ));
                }
            }
            true
        } else {
            false
        }
    });
}

async fn run_repo(idx: usize, cfg: &RepoConfig, status_tx: Arc<watch::Sender<AppStatus>>) {
    if cfg.repo_path.is_empty() {
        status_tx.send_if_modified(|s| {
            if let Some(rs) = s.repos.get_mut(idx) {
                *rs = RepoStatus::new_unconfigured(cfg.repo_path.clone());
                true
            } else {
                false
            }
        });
        // Park until aborted.
        std::future::pending::<()>().await;
        return;
    }

    let wm = WatchManager::new(
        &cfg.repo_path,
        build_sync_config(cfg),
        build_watch_config(cfg),
    );

    let wm_handle = wm.handle();

    let push_fut = async {
        while let Some(snap) = wm_handle.recv().await {
            status_tx.send_if_modified(|app_status| {
                let Some(rs) = app_status.repos.get_mut(idx) else {
                    return false;
                };

                if snap.is_syncing {
                    rs.sync_state_label = "Syncing…".to_string();
                    rs.sync_state_id = "syncing".to_string();
                    rs.is_syncing = true;
                } else {
                    rs.is_syncing = false;
                    if let Some(ref st) = snap.last_sync_state {
                        rs.sync_state_label = sync_state_label(st);
                        rs.sync_state_id = sync_state_id(st).to_string();
                    }
                }

                rs.last_sync_time = snap.last_successful_sync.or(rs.last_sync_time);

                if let Some(ref repo_st) = snap.last_repo_state {
                    rs.repo_state_label = repo_state_label(repo_st).to_string();
                }

                rs.error = snap.last_error.as_ref().map(SyncErrorPayload::from);

                true
            });
        }
    };

    let watch_result = tokio::select! {
        r = wm.watch().instrument(tracing::info_span!("repo_watcher", repo = %cfg.repo_path)) => Some(r),
        _ = push_fut => None,
    };

    if let Some(Err(e)) = watch_result {
        status_tx.send_if_modified(|app_status| {
            if let Some(rs) = app_status.repos.get_mut(idx) {
                rs.error = Some(SyncErrorPayload::from(
                    &git_sync_lib::SyncErrorSummary::from(&e),
                ));
                true
            } else {
                false
            }
        });
    }
}
