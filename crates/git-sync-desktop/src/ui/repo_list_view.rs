use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    table::{Column, Table, TableDelegate, TableState},
    v_flex, ActiveTheme, Sizable,
};

use super::nav_state::{NavRequest, NavState};
use crate::state::AppState;
use crate::status::format_last_sync;

pub fn dot_color(state_id: &str) -> Hsla {
    match state_id {
        "equal" => hsla(0.33, 0.59, 0.40, 1.0),
        "ahead" | "behind" => hsla(0.09, 1.0, 0.50, 1.0),
        "diverged" | "error" => hsla(0.0, 0.74, 0.58, 1.0),
        "syncing" => hsla(0.59, 0.80, 0.54, 1.0),
        _ => hsla(0.0, 0.0, 0.62, 1.0),
    }
}

pub struct RepoTableDelegate {
    state: Entity<AppState>,
    nav: Entity<NavState>,
    columns: Vec<Column>,
}

impl RepoTableDelegate {
    fn new(state: Entity<AppState>, nav: Entity<NavState>) -> Self {
        let columns = vec![
            Column::new("name", "Repository")
                .width(px(240.0))
                .resizable(false)
                .movable(false),
            Column::new("status", "Status")
                .width(px(160.0))
                .resizable(false)
                .movable(false),
            Column::new("last_sync", "Last Sync")
                .width(px(120.0))
                .resizable(false)
                .movable(false),
            Column::new("actions", "")
                .width(px(90.0))
                .resizable(false)
                .movable(false)
                .selectable(false),
        ];
        Self {
            state,
            nav,
            columns,
        }
    }
}

impl TableDelegate for RepoTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        4
    }

    fn rows_count(&self, cx: &App) -> usize {
        self.state.read(cx).config.repositories.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let s = self.state.read(cx);
        let repo = s.config.repositories.get(row_ix).cloned();
        let status = s.status.repos.get(row_ix).cloned();
        let _ = s;

        let cell: AnyElement = match col_ix {
            0 => {
                let name = repo.as_ref().map(|r| r.display_name()).unwrap_or_default();
                let path = repo
                    .as_ref()
                    .map(|r| r.repo_path.clone())
                    .unwrap_or_default();
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child(path),
                    )
                    .into_any_element()
            }
            1 => {
                let state_id = status
                    .as_ref()
                    .map(|s| s.sync_state_id.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let label = status
                    .as_ref()
                    .map(|s| s.sync_state_label.clone())
                    .unwrap_or_default();
                let color = dot_color(&state_id);
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .child(div().size_2().rounded_full().bg(color))
                    .child(div().text_sm().child(label))
                    .into_any_element()
            }
            2 => {
                let last_sync = format_last_sync(status.as_ref().and_then(|s| s.last_sync_time));
                div()
                    .text_sm()
                    .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                    .child(last_sync)
                    .into_any_element()
            }
            _ => {
                let state = self.state.clone();
                let nav = self.nav.clone();

                h_flex()
                    .gap_1()
                    .child(
                        Button::new(SharedString::from(format!("sync-{row_ix}")))
                            .ghost()
                            .small()
                            .label("↺")
                            .cursor_pointer()
                            .on_click(move |_, _, cx| {
                                state.update(cx, |s, _| s.sync_now(row_ix));
                            }),
                    )
                    .child(
                        Button::new(SharedString::from(format!("settings-{row_ix}")))
                            .ghost()
                            .small()
                            .label("⚙")
                            .cursor_pointer()
                            .on_click(move |_, _, cx| {
                                nav.update(cx, |n, cx| {
                                    n.request = Some(NavRequest::OpenSettings(Some(row_ix)));
                                    cx.notify();
                                });
                            }),
                    )
                    .into_any_element()
            }
        };

        cell
    }

    fn render_empty(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
            .child("No repositories configured")
    }
}

pub struct RepoListView {
    nav: Entity<NavState>,
    table_state: Entity<TableState<RepoTableDelegate>>,
    _sub: Subscription,
}

impl RepoListView {
    pub fn new(
        state: Entity<AppState>,
        nav: Entity<NavState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let delegate = RepoTableDelegate::new(state.clone(), nav.clone());
        let table_state = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .row_selectable(false)
                .col_selectable(false)
                .sortable(false)
                .col_movable(false)
                .col_resizable(false)
        });

        let ts = table_state.clone();
        let sub = cx.observe(&state, move |_, _, cx| {
            ts.update(cx, |s, cx| s.refresh(cx));
            cx.notify();
        });

        Self {
            nav,
            table_state,
            _sub: sub,
        }
    }
}

impl Render for RepoListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let nav = self.nav.clone();

        v_flex()
            .size_full()
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(Table::new(&self.table_state).large()),
            )
            .child(
                div()
                    .p_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("add-repo")
                            .ghost()
                            .label("＋ Add Repository")
                            .cursor_pointer()
                            .on_click(move |_, _, cx| {
                                nav.update(cx, |n, cx| {
                                    n.request = Some(NavRequest::OpenSettings(None));
                                    cx.notify();
                                });
                            }),
                    ),
            )
    }
}
