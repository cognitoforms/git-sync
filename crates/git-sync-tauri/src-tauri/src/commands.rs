use std::sync::Mutex;

use tauri::State;

use crate::config::DesktopConfig;
use crate::log_layer::FrontendLogEntry;
use crate::status::AppStatus;
use crate::worker::BgCmd;
use crate::{AppState, LogState, StatusState};

#[tauri::command]
pub fn get_config(state: State<'_, Mutex<AppState>>) -> DesktopConfig {
    state.lock().unwrap().config.clone()
}

#[tauri::command]
pub fn get_status(state: State<'_, Mutex<StatusState>>) -> AppStatus {
    state.lock().unwrap().0.borrow().clone()
}

#[tauri::command]
pub fn set_config(
    state: State<'_, Mutex<AppState>>,
    config: DesktopConfig,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    crate::config::save_config(&config).map_err(|e| e.to_string())?;
    s.worker_tx
        .send(BgCmd::Reconfigure(config.clone()))
        .map_err(|e| e.to_string())?;
    s.config = config;
    Ok(())
}

#[tauri::command]
pub fn sync_now(state: State<'_, Mutex<AppState>>, index: usize) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .worker_tx
        .send(BgCmd::SyncNow(index))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_repo_path(path: String) -> bool {
    std::path::Path::new(&path).join(".git").exists()
}

#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |path_opt| {
        let _ = tx.send(path_opt);
    });
    rx.await
        .map_err(|e| e.to_string())
        .map(|path_opt| {
            path_opt.and_then(|p| p.into_path().ok())
                    .map(|pb| pb.to_string_lossy().to_string())
        })
}

#[tauri::command]
pub fn get_log_history(
    repo: Option<String>,
    state: State<'_, LogState>,
) -> Vec<FrontendLogEntry> {
    let hist = state.history.lock().unwrap();
    hist.iter()
        .filter(|e| match &repo {
            None => true,
            Some(r) => e.repo.as_deref() == Some(r.as_str()),
        })
        .cloned()
        .collect()
}
