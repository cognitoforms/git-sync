mod assets;
mod config;
mod state;
mod status;
mod tray;
mod ui;
mod worker;

use std::sync::Arc;
use std::time::Duration;

use gpui::*;
use gpui_component::{Root, TitleBar};
use tray_icon::{
    menu::{Menu as TrayMenu, MenuEvent, MenuItem as TrayMenuItem},
    TrayIconBuilder, TrayIconEvent,
};

actions!(git_sync, [Quit, About]);

use config::load_config;
use state::AppState;
use status::AppStatus;
use tray::create_tray_icon;
use ui::AppWindow;
use worker::{run_background, BgCmd};
use tokio::sync::watch;

struct AboutWindow;

impl Render for AboutWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(div().text_xl().font_weight(FontWeight::BOLD).child("git-sync"))
            .child(div().text_sm().child(format!("Version {}", env!("CARGO_PKG_VERSION"))))
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .pretty()
        .init();

    #[cfg(target_os = "linux")]
    gtk::init().expect("Failed to initialize GTK");

    let config = load_config();
    let (status_tx, status_rx) = watch::channel(AppStatus::default());
    let status_tx = Arc::new(status_tx);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<BgCmd>();

    // Spawn background sync worker
    {
        let status_bg = Arc::clone(&status_tx);
        let init_cfg = config.clone();
        std::thread::spawn(move || run_background(init_cfg, status_bg, rx));
    }

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);

            // Shared state entity
            let state = cx.new(|_| AppState::new(config.clone(), tx.clone()));

            // Register action handlers
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.on_action(|_: &About, cx| {
                cx.open_window(
                    WindowOptions {
                        focus: true,
                        show: true,
                        window_min_size: Some(size(px(320.), px(180.))),
                        ..Default::default()
                    },
                    |window, cx| {
                        let view = cx.new(|_| AboutWindow);
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
                .ok();
            });

            // Set app menu bar
            cx.set_menus(vec![Menu {
                name: "git-sync".into(),
                items: vec![
                    MenuItem::action("About git-sync", About),
                    MenuItem::separator(),
                    MenuItem::action("Quit git-sync", Quit),
                ],
            }]);

            // System tray
            let quit_item = TrayMenuItem::new("Quit git-sync", true, None);
            let tray_menu = TrayMenu::new();
            tray_menu.append(&quit_item).expect("failed to append tray menu item");
            let _tray = TrayIconBuilder::new()
                .with_tooltip("git-sync")
                .with_icon(create_tray_icon())
                .with_menu(Box::new(tray_menu))
                .with_menu_on_left_click(false)
                .build()
                .expect("failed to build system tray icon");
            let quit_id = quit_item.id().clone();

            let state_task = state.clone();

            // Status listener: push AppStatus updates to GPUI whenever the worker
            // writes a new value — no polling, wakes only on change.
            cx.spawn({
                let state_task = state_task.clone();
                let mut rx = status_rx;
                async move |cx: &mut gpui::AsyncApp| {
                    loop {
                        if rx.changed().await.is_err() {
                            break;
                        }
                        let snapshot = rx.borrow_and_update().clone();
                        cx.update(|app| {
                            state_task.update(app, |s, cx| s.update_status(snapshot, cx));
                        })
                        .ok();
                    }
                }
            })
            .detach();

            cx.spawn(async move |cx| {
                // Move tray into task — keeps TrayIcon alive for app lifetime
                let _tray = _tray;

                let main_window_opts = || WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_min_size: Some(size(px(520.), px(440.))),
                    focus: true,
                    show: true,
                    ..Default::default()
                };

                let mut window = Some(cx.open_window(
                    main_window_opts(),
                    {
                        let state = state_task.clone();
                        move |window, cx| {
                            let view = cx.new(|cx| AppWindow::new(state.clone(), window, cx));
                            cx.new(|cx| Root::new(view, window, cx))
                        }
                    },
                )?);

                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;

                    // GTK event pump (Linux only)
                    #[cfg(target_os = "linux")]
                    cx.update(|_| {
                        while gtk::events_pending() {
                            gtk::main_iteration_do(false);
                        }
                    })
                    .ok();

                    // Detect user-closed window (X button)
                    if let Some(w) = window {
                        let alive = cx
                            .update(|app| app.update_window(w.into(), |_, _, _| {}).is_ok())
                            .unwrap_or(false);
                        if !alive {
                            window = None;
                        }
                    }

                    // Tray icon left-click: close if open, reopen if closed
                    while let Ok(TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    }) = TrayIconEvent::receiver().try_recv()
                    {
                        if let Some(w) = window.take() {
                            cx.update(|app| {
                                app.update_window(w.into(), |_, win, _| win.remove_window()).ok();
                            })
                            .ok();
                        } else if let Ok(w) = cx.open_window(
                            main_window_opts(),
                            {
                                let state = state_task.clone();
                                move |window, cx| {
                                    let view =
                                        cx.new(|cx| AppWindow::new(state.clone(), window, cx));
                                    cx.new(|cx| Root::new(view, window, cx))
                                }
                            },
                        ) {
                            window = Some(w);
                        }
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
