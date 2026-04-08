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
pub enum ConflictKindPayload {
    ContentConflict,
    DeletedByUs,
    DeletedByThem,
}

impl From<git_sync_lib::ConflictKind> for ConflictKindPayload {
    fn from(k: git_sync_lib::ConflictKind) -> Self {
        match k {
            git_sync_lib::ConflictKind::ContentConflict => Self::ContentConflict,
            git_sync_lib::ConflictKind::DeletedByUs => Self::DeletedByUs,
            git_sync_lib::ConflictKind::DeletedByThem => Self::DeletedByThem,
        }
    }
}

#[derive(Serialize, Clone, Debug, specta::Type)]
pub struct ConflictFileContentPayload {
    pub path: String,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub base: Option<String>,
    pub conflict_kind: ConflictKindPayload,
}

#[derive(Serialize, Deserialize, Clone, Debug, specta::Type)]
pub struct ResolvedFilePayload {
    pub path: String,
    pub content: String,
    pub deleted: bool,
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
    Ok(files
        .into_iter()
        .map(|f| ConflictFileContentPayload {
            path: f.path,
            ours: f.ours,
            theirs: f.theirs,
            base: f.base,
            conflict_kind: f.conflict_kind.into(),
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn complete_conflict_merge(
    state: State<'_, Mutex<AppState>>,
    index: usize,
    resolved: Vec<ResolvedFilePayload>,
) -> Result<(), String> {
    use git_sync_lib::ResolvedFileContent;
    let entries: Vec<ResolvedFileContent> = resolved
        .into_iter()
        .map(|r| ResolvedFileContent {
            path: r.path,
            content: r.content,
            deleted: r.deleted,
        })
        .collect();
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
