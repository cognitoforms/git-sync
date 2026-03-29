use std::sync::{Arc, Mutex};

use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputState},
};

/// A text input + "Browse…" button that lets the user pick a folder.
///
/// The OS file-picker runs on a background thread to avoid conflicts with
/// GPUI's Win32 message loop. The chosen path is staged in `pending` and
/// applied in `render()`, which has the `&mut Window` access that
/// `InputState::set_value` requires.
pub struct FileBrowserField {
    pub input: Entity<InputState>,
    pending: Arc<Mutex<Option<String>>>,
}

impl FileBrowserField {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        default_value: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> Self {
        let default_value = default_value.into();
        let placeholder = placeholder.into();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(default_value)
                .placeholder(placeholder)
        });
        Self {
            input,
            pending: Arc::new(Mutex::new(None)),
        }
    }

    /// Returns the current text value of the input.
    pub fn value(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }
}

impl Render for FileBrowserField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Apply any path chosen by the background file-picker thread.
        if let Some(path) = self.pending.lock().unwrap().take() {
            self.input.update(cx, |s, cx| {
                s.set_value(path, window, cx);
            });
        }

        let pending = Arc::clone(&self.pending);

        div()
            .flex()
            .gap_2()
            .items_center()
            .child(Input::new(&self.input).w_full())
            .child(
                Button::new("browse")
                    .ghost()
                    .label("Browse…")
                    .cursor_pointer()
                    .on_click(cx.listener(move |_, _, _, _cx| {
                        let pending = Arc::clone(&pending);
                        std::thread::spawn(move || {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                *pending.lock().unwrap() =
                                    Some(path.to_string_lossy().to_string());
                            }
                        });
                    })),
            )
    }
}
