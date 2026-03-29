use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

use crate::state::AppState;
use crate::status::format_last_sync;

pub struct StatusTab {
    state: Entity<AppState>,
    _sub: Subscription,
}

impl StatusTab {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let sub = cx.observe(&state, |_, _entity, cx| cx.notify());
        Self { state, _sub: sub }
    }

    fn dot_color(state_id: &str) -> Hsla {
        match state_id {
            "equal" => hsla(0.33, 0.59, 0.40, 1.0),      // green
            "ahead" | "behind" => hsla(0.09, 1.0, 0.50, 1.0), // orange
            "diverged" | "error" => hsla(0.0, 0.74, 0.58, 1.0), // red
            "syncing" => hsla(0.59, 0.80, 0.54, 1.0),     // blue
            _ => hsla(0.0, 0.0, 0.62, 1.0),               // gray
        }
    }
}

impl Render for StatusTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let s = self.state.read(cx);
        let status = s.status.clone();
        let repo_path = s.config.repo_path.clone();
        let _ = s;

        let dot_color = Self::dot_color(&status.sync_state_id);
        let state = self.state.clone();

        v_flex()
            .gap_3()
            .p_4()
            .size_full()
            // Repository path row
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Repository"),
                    )
                    .child(
                        div().text_sm().child(if repo_path.is_empty() {
                            "Not configured".to_string()
                        } else {
                            repo_path
                        }),
                    ),
            )
            // Repo state row
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Repository state:"),
                    )
                    .child(div().text_sm().child(status.repo_state_label.clone())),
            )
            // Sync state row with dot
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().size_2().rounded_full().bg(dot_color))
                    .child(div().text_sm().child(status.sync_state_label.clone())),
            )
            // Last sync row
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Last sync:"),
                    )
                    .child(div().text_sm().child(format_last_sync(status.last_sync_time))),
            )
            // Error box (conditional)
            .when(status.error.is_some(), |el: gpui::Div| {
                el.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .bg(hsla(0.0, 0.74, 0.58, 0.1))
                        .text_sm()
                        .text_color(hsla(0.0, 0.74, 0.58, 1.0))
                        .child(status.error.clone().unwrap_or_default()),
                )
            })
            // Sync Now button
            .child(
                Button::new("sync-now")
                    .primary()
                    .label("Sync Now")
                    .cursor_pointer()
                    .on_click(cx.listener(move |_, _, _, cx| {
                        state.update(cx, |s, _| s.sync_now());
                    })),
            )
    }
}
