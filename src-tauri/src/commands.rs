use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::config::DesktopConfig;
use crate::diff_view::{DiffCommitSummary, DiffCompareMode, DiffViewData};
use crate::log_layer::FrontendLogEntry;
use crate::status::AppStatus;
use crate::worker::BgCmd;
use crate::{AppState, DiffViewerContext, DiffViewerState, LogState, StatusState};

fn diff_viewer_label(repo_path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    repo_path.hash(&mut hasher);
    format!("diff-viewer-{:016x}", hasher.finish())
}

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

#[tauri::command]
#[specta::specta]
pub fn open_diff_viewer(
    app: tauri::AppHandle,
    repo_path: String,
    repo_name: String,
    state: State<'_, DiffViewerState>,
) -> Result<(), String> {
    let label = diff_viewer_label(&repo_path);

    state.contexts.lock().unwrap().insert(
        label.clone(),
        DiffViewerContext {
            repo_path,
            repo_name: repo_name.clone(),
        },
    );

    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let title = if repo_name.is_empty() {
        "Diff Viewer".to_string()
    } else {
        format!("{} — Diff Viewer", repo_name)
    };

    let win_builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::default())
        .title(title)
        .inner_size(1200.0, 780.0)
        .min_inner_size(720.0, 520.0)
        .shadow(true);

    #[cfg(target_os = "macos")]
    let win_builder = win_builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);

    #[cfg(not(target_os = "macos"))]
    let win_builder = win_builder.decorations(false);

    let window = win_builder.build().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_diff_viewer_context(
    window_label: String,
    state: State<'_, DiffViewerState>,
) -> Option<DiffViewerContext> {
    state.contexts.lock().unwrap().get(&window_label).cloned()
}

#[tauri::command]
#[specta::specta]
pub fn list_diff_commits(repo_path: String, limit: Option<usize>) -> Result<Vec<DiffCommitSummary>, String> {
    crate::diff_view::list_commits(&repo_path, limit.unwrap_or(200)).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_diff_view(
    repo_path: String,
    mode: DiffCompareMode,
    selected_file: Option<String>,
) -> Result<DiffViewData, String> {
    crate::diff_view::load_view(&repo_path, mode, selected_file.as_deref()).map_err(|e| e.to_string())
}
