use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::DesktopConfig;
use crate::log_layer::FrontendLogEntry;
use crate::status::AppStatus;
use crate::worker::BgCmd;
use crate::{AppState, LogState, StatusState};

#[tauri::command]
#[specta::specta]
pub fn get_config(state: State<'_, Mutex<AppState>>) -> DesktopConfig {
    state.lock().unwrap().config.clone()
}

#[tauri::command]
#[specta::specta]
pub fn get_status(state: State<'_, Mutex<StatusState>>) -> AppStatus {
    state.lock().unwrap().0.borrow().clone()
}

#[tauri::command]
#[specta::specta]
pub fn set_config(state: State<'_, Mutex<AppState>>, config: DesktopConfig) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    crate::config::save_config(&config).map_err(|e| e.to_string())?;
    s.worker_tx
        .send(BgCmd::Reconfigure(config.clone()))
        .map_err(|e| e.to_string())?;
    s.config = config;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn sync_now(state: State<'_, Mutex<AppState>>, index: usize) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .worker_tx
        .send(BgCmd::SyncNow(index))
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn validate_repo_path(path: String) -> bool {
    std::path::Path::new(&path).join(".git").exists()
}

#[tauri::command]
#[specta::specta]
pub async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |path_opt| {
        let _ = tx.send(path_opt);
    });
    rx.await.map_err(|e| e.to_string()).map(|path_opt| {
        path_opt
            .and_then(|p| p.into_path().ok())
            .map(|pb| pb.to_string_lossy().to_string())
    })
}

#[derive(Serialize, Clone, Debug, specta::Type)]
pub struct ConflictInfoPayload {
    pub conflicted_files: Vec<String>,
    pub on_conflict_branch: bool,
    pub conflict_branch_name: Option<String>,
    pub target_branch: String,
}

#[tauri::command]
#[specta::specta]
pub fn get_conflict_info(
    state: State<'_, Mutex<AppState>>,
    index: usize,
) -> Result<ConflictInfoPayload, String> {
    use git_sync_lib::RepositorySynchronizer;
    let config = {
        let s = state.lock().unwrap();
        s.config
            .repositories
            .get(index)
            .cloned()
            .ok_or_else(|| format!("No repository at index {}", index))?
    };
    let sync_config = crate::worker::build_sync_config_pub(&config);
    let syncer = RepositorySynchronizer::new_with_detected_branch(&config.repo_path, sync_config)
        .map_err(|e| e.to_string())?;

    let conflicted_files = syncer.get_conflict_info().map_err(|e| e.to_string())?;
    let on_conflict_branch = syncer.is_on_fallback_branch().unwrap_or(false);
    let conflict_branch_name = syncer.get_conflict_branch();
    let target_branch = syncer
        .get_target_branch()
        .unwrap_or_else(|_| "main".to_string());

    Ok(ConflictInfoPayload {
        conflicted_files,
        on_conflict_branch,
        conflict_branch_name,
        target_branch,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategyPayload {
    KeepMine,
    AcceptRemote,
    AbandonConflictBranch,
}

impl From<ConflictResolutionStrategyPayload> for crate::worker::ConflictResolutionStrategy {
    fn from(p: ConflictResolutionStrategyPayload) -> Self {
        match p {
            ConflictResolutionStrategyPayload::KeepMine => Self::KeepMine,
            ConflictResolutionStrategyPayload::AcceptRemote => Self::AcceptRemote,
            ConflictResolutionStrategyPayload::AbandonConflictBranch => Self::AbandonConflictBranch,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn resolve_conflict(
    state: State<'_, Mutex<AppState>>,
    index: usize,
    strategy: ConflictResolutionStrategyPayload,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .worker_tx
        .send(BgCmd::ResolveConflict {
            index,
            strategy: strategy.into(),
        })
        .map_err(|e| e.to_string())
}

#[derive(Serialize, Clone, Debug, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConflictFileContentPayload {
    Content {
        path: String,
        their_path: Option<String>,
        ours: String,
        theirs: String,
        base: Option<String>,
    },
    DeletedByUs {
        path: String,
        theirs: String,
        base: Option<String>,
    },
    DeletedByThem {
        path: String,
        ours: String,
        base: Option<String>,
    },
    RenameRename {
        our_path: String,
        their_path: String,
        ours: String,
        theirs: String,
        base: Option<String>,
    },
}

impl From<git_sync_lib::ConflictFileContent> for ConflictFileContentPayload {
    fn from(f: git_sync_lib::ConflictFileContent) -> Self {
        use git_sync_lib::ConflictFileContent as C;
        match f {
            C::Content {
                path,
                their_path,
                ours,
                theirs,
                base,
            } => Self::Content {
                path,
                their_path,
                ours,
                theirs,
                base,
            },
            C::DeletedByUs { path, theirs, base } => Self::DeletedByUs { path, theirs, base },
            C::DeletedByThem { path, ours, base } => Self::DeletedByThem { path, ours, base },
            C::RenameRename {
                our_path,
                their_path,
                ours,
                theirs,
                base,
            } => Self::RenameRename {
                our_path,
                their_path,
                ours,
                theirs,
                base,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResolvedFilePayload {
    Written {
        path: String,
        content: String,
    },
    Deleted {
        path: String,
    },
    RenameResolved {
        chosen_path: String,
        discarded_path: String,
        content: String,
    },
}

impl From<ResolvedFilePayload> for git_sync_lib::ResolvedFileContent {
    fn from(r: ResolvedFilePayload) -> Self {
        use git_sync_lib::ResolvedFileContent as R;
        match r {
            ResolvedFilePayload::Written { path, content } => R::Written { path, content },
            ResolvedFilePayload::Deleted { path } => R::Deleted { path },
            ResolvedFilePayload::RenameResolved {
                chosen_path,
                discarded_path,
                content,
            } => R::RenameResolved {
                chosen_path,
                discarded_path,
                content,
            },
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_conflict_files_content(
    state: State<'_, Mutex<AppState>>,
    index: usize,
) -> Result<Vec<ConflictFileContentPayload>, String> {
    use git_sync_lib::RepositorySynchronizer;
    let config = {
        let s = state.lock().unwrap();
        s.config
            .repositories
            .get(index)
            .cloned()
            .ok_or_else(|| format!("No repository at index {}", index))?
    };
    let sync_config = crate::worker::build_sync_config_pub(&config);
    let syncer = RepositorySynchronizer::new_with_detected_branch(&config.repo_path, sync_config)
        .map_err(|e| e.to_string())?;
    let files = syncer
        .get_conflict_files_content()
        .map_err(|e| e.to_string())?;
    Ok(files.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[specta::specta]
pub fn complete_conflict_merge(
    state: State<'_, Mutex<AppState>>,
    index: usize,
    resolved: Vec<ResolvedFilePayload>,
) -> Result<(), String> {
    use git_sync_lib::ResolvedFileContent;
    let entries: Vec<ResolvedFileContent> = resolved.into_iter().map(Into::into).collect();
    state
        .lock()
        .unwrap()
        .worker_tx
        .send(BgCmd::CompleteMerge {
            index,
            resolved: entries,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_log_history(repo: Option<String>, state: State<'_, LogState>) -> Vec<FrontendLogEntry> {
    let hist = state.history.lock().unwrap();
    hist.iter()
        .filter(|e| match &repo {
            None => true,
            Some(r) => e.repo.as_deref() == Some(r.as_str()),
        })
        .cloned()
        .collect()
}
