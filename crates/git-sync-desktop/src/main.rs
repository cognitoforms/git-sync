mod config;
mod status;
mod tray;
mod worker;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};

use config::{load_config, save_config};
use status::{format_last_sync, AppStatus};
use tray::create_tray_icon;
use worker::{run_background, BgCmd};

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    gtk::init()?;

    let config = load_config();
    let status: Arc<Mutex<AppStatus>> = Arc::new(Mutex::new(AppStatus::default()));

    // Spawn background worker with a tokio unbounded channel.
    // UnboundedSender::send() is sync, so it works fine from Slint callbacks.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<BgCmd>();
    {
        let status_bg = Arc::clone(&status);
        let init_cfg = config.clone();
        std::thread::spawn(move || run_background(init_cfg, status_bg, rx));
    }

    let window = AppWindow::new()?;

    // Populate UI from persisted config
    window.set_config(config.into());

    window
        .window()
        .on_close_requested(|| slint::CloseRequestResponse::HideWindow);

    // ── Tray ──────────────────────────────────────────────────────────────────
    let quit_item = MenuItem::new("Quit git-sync", true, None);
    let tray_menu = Menu::new();
    tray_menu.append(&quit_item)?;
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("git-sync")
        .with_icon(create_tray_icon())
        .with_menu(Box::new(tray_menu))
        .build()?;

    // ── Callbacks ─────────────────────────────────────────────────────────────
    let win_browse = window.as_weak();
    window.on_browse_repo(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(win) = win_browse.upgrade() {
                let mut cfg = win.get_config();
                cfg.repo_path = path.to_string_lossy().to_string().into();
                win.set_config(cfg);
            }
        }
    });

    let tx_sync = tx.clone();
    window.on_sync_now(move || {
        let _ = tx_sync.send(BgCmd::SyncNow);
    });

    let tx_cfg = tx.clone();
    let win_weak = window.as_weak();
    window.on_save_config(move || {
        let Some(win) = win_weak.upgrade() else {
            return;
        };
        let ui = win.get_config();
        let new_cfg = config::DesktopConfig {
            repo_path: ui.repo_path.to_string(),
            remote: {
                let r = ui.remote.to_string();
                if r.is_empty() {
                    "origin".to_string()
                } else {
                    r
                }
            },
            branch: ui.branch.to_string(),
            interval_secs: ui.interval.max(10) as u64,
            sync_new_files: ui.sync_new_files,
            skip_hooks: ui.skip_hooks,
            conflict_branch: ui.conflict_branch,
            commit_message: ui.commit_message.to_string(),
        };
        if let Err(e) = save_config(&new_cfg) {
            eprintln!("Failed to save config: {}", e);
        }
        let _ = tx_cfg.send(BgCmd::Reconfigure(new_cfg));
    });

    // ── Status polling timer (500 ms) ─────────────────────────────────────────
    let status_poll = Arc::clone(&status);
    let win_poll = window.as_weak();
    let poll_timer = slint::Timer::default();
    poll_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(500),
        move || {
            let Some(win) = win_poll.upgrade() else {
                return;
            };
            let s = status_poll.lock().unwrap();
            let sync_status = if s.is_syncing {
                SyncStatus::Syncing
            } else {
                match s.sync_state_id.as_str() {
                    "equal"       => SyncStatus::Equal,
                    "ahead"       => SyncStatus::Ahead,
                    "behind"      => SyncStatus::Behind,
                    "diverged"    => SyncStatus::Diverged,
                    "error"       => SyncStatus::Error,
                    "no-upstream" => SyncStatus::NoUpstream,
                    _             => SyncStatus::Unknown,
                }
            };
            win.set_sync_state_label(s.sync_state_label.clone().into());
            win.set_sync_status(sync_status);
            win.set_repo_state_label(s.repo_state_label.clone().into());
            win.set_error_message(s.error.clone().unwrap_or_default().into());
            win.set_last_sync_label(format_last_sync(s.last_sync_time).into());
        },
    );

    // ── Tray/menu event polling timer (50 ms) ─────────────────────────────────
    let visible = Arc::new(AtomicBool::new(true));
    let visible_tray = Arc::clone(&visible);
    let win_tray = window.as_weak();
    let quit_id = quit_item.id().clone();

    let tray_timer = slint::Timer::default();
    tray_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            #[cfg(target_os = "linux")]
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }

            if let Ok(TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            }) = TrayIconEvent::receiver().try_recv()
            {
                if let Some(win) = win_tray.upgrade() {
                    if visible_tray.load(Ordering::Relaxed) {
                        let _ = win.hide();
                        visible_tray.store(false, Ordering::Relaxed);
                    } else {
                        let _ = win.show();
                        visible_tray.store(true, Ordering::Relaxed);
                    }
                }
            }

            if let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id == quit_id {
                    slint::quit_event_loop().unwrap();
                }
            }
        },
    );

    window.run()?;
    Ok(())
}
