mod config;
mod state;
mod status;
mod tray;
mod ui;
mod worker;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::*;
use gpui_component::{Root, TitleBar};
use gpui_component_assets::Assets;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};

use config::load_config;
use state::AppState;
use status::AppStatus;
use tray::create_tray_icon;
use ui::AppWindow;
use worker::{run_background, BgCmd};

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .pretty()
        .init();

    #[cfg(target_os = "linux")]
    gtk::init().expect("Failed to initialize GTK");

    let config = load_config();
    let status: Arc<Mutex<AppStatus>> = Arc::new(Mutex::new(AppStatus::default()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<BgCmd>();

    // Spawn background sync worker
    {
        let status_bg = Arc::clone(&status);
        let init_cfg = config.clone();
        std::thread::spawn(move || run_background(init_cfg, status_bg, rx));
    }

    Application::new()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);

            // Shared state entity
            let state = cx.new(|_| AppState::new(config.clone(), tx.clone()));

            // System tray
            let quit_item = MenuItem::new("Quit git-sync", true, None);
            let tray_menu = Menu::new();
            tray_menu.append(&quit_item).unwrap();
            let _tray = TrayIconBuilder::new()
                .with_tooltip("git-sync")
                .with_icon(create_tray_icon())
                .with_menu(Box::new(tray_menu))
                .build()
                .unwrap();
            let quit_id = quit_item.id().clone();

            let state_task = state.clone();
            let status_task = Arc::clone(&status);

            cx.spawn(async move |cx| {
                let window = cx.open_window(
                    WindowOptions {
                        titlebar: Some(TitleBar::title_bar_options()),
                        window_min_size: Some(size(px(520.), px(440.))),
                        focus: true,
                        show: true,
                        ..Default::default()
                    },
                    |window, cx| {
                        let view = cx.new(|cx| AppWindow::new(state_task.clone(), window, cx));
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )?;

                let mut tick: u32 = 0;
                let mut visible = true;

                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                    tick += 1;

                    // Push status update every 500 ms (10 ticks)
                    if tick.is_multiple_of(10) {
                        let snapshot = status_task.lock().unwrap().clone();
                        cx.update(|app| {
                            state_task.update(app, |s, cx| {
                                s.update_status(snapshot, cx);
                            });
                        })
                        .ok();
                    }

                    // GTK event pump (Linux only)
                    #[cfg(target_os = "linux")]
                    cx.update(|_| {
                        while gtk::events_pending() {
                            gtk::main_iteration_do(false);
                        }
                    })
                    .ok();

                    // Tray icon click: toggle window visibility
                    while let Ok(TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    }) = TrayIconEvent::receiver().try_recv()
                    {
                        cx.update(|app| {
                            app.update_window(window.into(), |_, win, _| {
                                if visible {
                                    win.minimize_window();
                                } else {
                                    win.activate_window();
                                }
                            })
                            .ok();
                        })
                        .ok();
                        visible = !visible;
                    }

                    // Tray menu events
                    while let Ok(event) = MenuEvent::receiver().try_recv() {
                        if event.id == quit_id {
                            cx.update(|app| app.quit()).ok();
                            return Ok::<_, anyhow::Error>(());
                        }
                    }
                }
            })
            .detach();
        });
}
