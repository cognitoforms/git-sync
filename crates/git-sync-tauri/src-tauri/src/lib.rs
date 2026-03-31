mod commands;
mod config;
mod log_layer;
mod status;
mod worker;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use tauri::LogicalPosition;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::watch;
use tracing_subscriber::prelude::*;

use config::{load_config, DesktopConfig};
use log_layer::{FrontendLogEntry, TauriLogLayer};
use status::AppStatus;
use worker::BgCmd;

pub struct AppState {
    pub config: DesktopConfig,
    pub worker_tx: tokio::sync::mpsc::UnboundedSender<BgCmd>,
}

pub struct StatusState(watch::Receiver<AppStatus>);

pub struct LogState {
    pub history: Arc<Mutex<VecDeque<FrontendLogEntry>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (log_tx, mut log_rx) = tokio::sync::mpsc::unbounded_channel::<FrontendLogEntry>();
    let log_history = Arc::new(Mutex::new(VecDeque::<FrontendLogEntry>::new()));
    let log_history_for_task = Arc::clone(&log_history);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(tracing_subscriber::EnvFilter::from_default_env()),
        )
        .with(TauriLogLayer::new(log_tx))
        .init();

    let config = load_config();
    let (status_tx, status_rx) = watch::channel(AppStatus::default());
    let status_tx = Arc::new(status_tx);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<BgCmd>();

    {
        let status_bg = Arc::clone(&status_tx);
        let init_cfg = config.clone();
        std::thread::spawn(move || worker::run_background(init_cfg, status_bg, cmd_rx));
    }

    let status_rx_forwarder = status_rx.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState {
            config,
            worker_tx: cmd_tx,
        }))
        .manage(Mutex::new(StatusState(status_rx)))
        .manage(LogState { history: log_history })
        .setup(|app| {
            // Build main window with platform-specific title bar style.
            let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Git Sync")
                .inner_size(680.0, 520.0)
                .min_inner_size(520.0, 440.0)
                .shadow(true);

            #[cfg(target_os = "macos")]
            let win_builder = win_builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .traffic_light_position(LogicalPosition::new(13, 22))
                .hidden_title(true);

            #[cfg(not(target_os = "macos"))]
            let win_builder = win_builder.decorations(false);

            win_builder.build()?;

            // Push status updates to the frontend whenever the worker sends a new snapshot.
            let app_handle = app.handle().clone();
            let mut rx = status_rx_forwarder;
            tauri::async_runtime::spawn(async move {
                loop {
                    if rx.changed().await.is_err() {
                        break;
                    }
                    let snapshot = rx.borrow_and_update().clone();
                    let _ = app_handle.emit("status-update", &snapshot);
                }
            });

            // Forward log entries to the frontend and accumulate history.
            let app_handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match log_rx.recv().await {
                        Some(entry) => {
                            {
                                let mut hist = log_history_for_task.lock().unwrap();
                                hist.push_back(entry.clone());
                                if hist.len() > 1000 {
                                    hist.pop_front();
                                }
                            }
                            let _ = app_handle2.emit("log-entry", &entry);
                        }
                        None => break,
                    }
                }
            });

            // System tray
            use tauri::{
                menu::{Menu, MenuItem},
                tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
            };

            let quit_item = MenuItem::with_id(app, "quit", "Quit Git Sync", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                window.hide().unwrap();
                            } else {
                                window.show().unwrap();
                                window.set_focus().unwrap();
                            }
                        }
                    }
                })
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::get_status,
            commands::set_config,
            commands::sync_now,
            commands::pick_folder,
            commands::validate_repo_path,
            commands::get_log_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
