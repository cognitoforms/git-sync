use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    h_flex,
    tab::{Tab, TabBar},
    v_flex,
    TitleBar,
};

use super::{settings_tab::SettingsTab, status_tab::StatusTab};
use crate::state::AppState;

pub struct AppWindow {
    state: Entity<AppState>,
    active_tab: usize,
    status_tab: Entity<StatusTab>,
    settings_tab: Entity<SettingsTab>,
    _sub: Subscription,
}

impl AppWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sub = cx.observe(&state, |_, _entity, cx| cx.notify());
        let status_tab = cx.new(|cx| StatusTab::new(state.clone(), cx));
        let settings_tab = cx.new(|cx| SettingsTab::new(state.clone(), window, cx));
        Self {
            state,
            active_tab: 0,
            status_tab,
            settings_tab,
            _sub: sub,
        }
    }

    fn dot_color(state_id: &str) -> Hsla {
        match state_id {
            "equal" => hsla(0.33, 0.59, 0.40, 1.0),
            "ahead" | "behind" => hsla(0.09, 1.0, 0.50, 1.0),
            "diverged" | "error" => hsla(0.0, 0.74, 0.58, 1.0),
            "syncing" => hsla(0.59, 0.80, 0.54, 1.0),
            _ => hsla(0.0, 0.0, 0.62, 1.0),
        }
    }
}

impl Render for AppWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let s = self.state.read(cx);
        let dot_color = Self::dot_color(&s.status.sync_state_id);
        let sync_label = s.status.sync_state_label.clone();
        let _ = s;

        v_flex()
            .size_full()
            // Title bar (handles drag region and native window controls)
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().size_2().rounded_full().bg(dot_color))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("git-sync"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                                .child(sync_label),
                        ),
                ),
            )
            // Tab bar
            .child(
                TabBar::new("main-tabs")
                    .selected_index(self.active_tab)
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.active_tab = *index;
                        cx.notify();
                    }))
                    .child(Tab::new().label("Status").cursor_pointer())
                    .child(Tab::new().label("Settings").cursor_pointer()),
            )
            // Tab content
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(self.active_tab == 0, |el| el.child(self.status_tab.clone()))
                    .when(self.active_tab == 1, |el| {
                        el.child(self.settings_tab.clone())
                    }),
            )
    }
}
