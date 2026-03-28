use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};

slint::include_modules!();

/// Build a simple 32×32 circle icon from raw RGBA bytes.
fn create_tray_icon() -> tray_icon::Icon {
    const SIZE: u32 = 32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let cx = SIZE as f32 / 2.0;
    let cy = SIZE as f32 / 2.0;
    let r = SIZE as f32 / 2.0 - 1.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let idx = ((y * SIZE + x) * 4) as usize;
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r * r {
                rgba[idx] = 0x4a; // R
                rgba[idx + 1] = 0x90; // G
                rgba[idx + 2] = 0xd4; // B
                rgba[idx + 3] = 0xff; // A
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, SIZE, SIZE).expect("Failed to create tray icon")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // GTK must be initialised on the main thread before creating a TrayIcon on Linux.
    #[cfg(target_os = "linux")]
    gtk::init()?;

    let window = AppWindow::new()?;

    // Hide instead of exit when the user clicks the window's close button.
    window.window().on_close_requested(|| slint::CloseRequestResponse::HideWindow);

    // Build the right-click context menu.
    let quit_item = MenuItem::new("Quit git-sync", true, None);
    let tray_menu = Menu::new();
    tray_menu.append(&quit_item)?;

    // Keep the tray icon alive for the duration of the program.
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("git-sync")
        .with_icon(create_tray_icon())
        .with_menu(Box::new(tray_menu))
        .build()?;

    // Shared visibility flag so the closure can track the current state.
    let visible = Arc::new(AtomicBool::new(true));
    let visible_clone = Arc::clone(&visible);

    let window_handle = window.as_weak();
    let quit_id = quit_item.id().clone();

    // Poll tray and menu events on every tick of a short-interval timer so
    // that they are handled on the Slint main thread.
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50),
        move || {
            // Let GTK process its own pending events (required for libappindicator).
            #[cfg(target_os = "linux")]
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }

            // Left-click → toggle window visibility.
            if let Ok(TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            }) = TrayIconEvent::receiver().try_recv()
            {
                if let Some(window) = window_handle.upgrade() {
                    if visible_clone.load(Ordering::Relaxed) {
                        let _ = window.hide();
                        visible_clone.store(false, Ordering::Relaxed);
                    } else {
                        let _ = window.show();
                        visible_clone.store(true, Ordering::Relaxed);
                    }
                }
            }

            // Context-menu "Quit" item → stop the event loop.
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
