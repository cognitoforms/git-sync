use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, IconName, TitleBar,
};

use super::nav_state::{NavRequest, NavState};
use super::repo_list_view::{dot_color, RepoListView};
use super::repo_settings_view::RepoSettingsView;
use crate::state::AppState;

pub enum AppView {
    RepoList,
    RepoSettings { idx: Option<usize> },
}

// Pending navigation that requires `&mut Window` to resolve (entity creation).
enum PendingNav {
    Settings(Option<usize>),
}

pub struct AppWindow {
    state: Entity<AppState>,
    nav: Entity<NavState>,
    view: AppView,
    repo_list: Entity<RepoListView>,
    repo_settings: Option<Entity<RepoSettingsView>>,
    pending_navigate: Option<PendingNav>,
    _state_sub: Subscription,
    _nav_sub: Subscription,
}

impl AppWindow {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let nav = cx.new(|_| NavState::new());

        let state_sub = cx.observe(&state, |_, _, cx| cx.notify());

        let repo_list = cx.new(|cx| RepoListView::new(state.clone(), nav.clone(), window, cx));

        let nav_entity = nav.clone();
        let nav_sub = cx.observe(&nav_entity, |this, nav_e, cx| {
            let req = nav_e.read(cx).request.clone();
            if let Some(req) = req {
                nav_e.update(cx, |n, _| n.request = None);
                match req {
                    NavRequest::OpenSettings(idx) => {
                        // Defer entity creation to render() where &mut Window is available.
                        this.pending_navigate = Some(PendingNav::Settings(idx));
                        cx.notify();
                    }
                    NavRequest::Back => {
                        this.repo_settings = None;
                        this.view = AppView::RepoList;
                        cx.notify();
                    }
                }
            }
        });

        Self {
            state,
            nav,
            view: AppView::RepoList,
            repo_list,
            repo_settings: None,
            pending_navigate: None,
            _state_sub: state_sub,
            _nav_sub: nav_sub,
        }
    }

    /// Aggregate the worst status across all repos for the title bar.
    fn aggregate_status_id(s: &crate::status::AppStatus) -> &'static str {
        let priority = |id: &str| match id {
            "error" => 5,
            "diverged" => 4,
            "syncing" => 3,
            "ahead" | "behind" => 2,
            "equal" => 1,
            _ => 0,
        };
        s.repos
            .iter()
            .max_by_key(|r| priority(r.sync_state_id.as_str()))
            .map(|r| match r.sync_state_id.as_str() {
                "error" => "error",
                "diverged" => "diverged",
                "syncing" => "syncing",
                "ahead" => "ahead",
                "behind" => "behind",
                "equal" => "equal",
                _ => "unknown",
            })
            .unwrap_or("unknown")
    }
}

impl Render for AppWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Process any pending navigation that requires window access.
        if let Some(pending) = self.pending_navigate.take() {
            match pending {
                PendingNav::Settings(idx) => {
                    let state = self.state.clone();
                    let nav = self.nav.clone();
                    let settings = cx.new(|cx| RepoSettingsView::new(state, nav, idx, window, cx));
                    self.repo_settings = Some(settings);
                    self.view = AppView::RepoSettings { idx };
                }
            }
        }

        let s = self.state.read(cx);
        let agg_id = Self::aggregate_status_id(&s.status);
        let agg_label = s
            .status
            .repos
            .iter()
            .find(|r| r.sync_state_id == agg_id)
            .map(|r| r.sync_state_label.clone())
            .unwrap_or_else(|| "No repositories".to_string());
        let dot = dot_color(agg_id);
        let in_settings = matches!(self.view, AppView::RepoSettings { .. });
        let title = match &self.view {
            AppView::RepoSettings { idx: None } => "Add Repository",
            AppView::RepoSettings { idx: Some(_) } => "Repo Settings",
            AppView::RepoList => "git-sync",
        };
        let _ = s;

        let nav = self.nav.clone();

        v_flex()
            .size_full()
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        // Back button (settings view only)
                        .when(in_settings, |el| {
                            el.child(
                                Button::new("back")
                                    .ghost()
                                    .icon(IconName::ArrowLeft)
                                    .cursor_pointer()
                                    .on_click(move |_, _, cx| {
                                        nav.update(cx, |n, cx| {
                                            n.request = Some(NavRequest::Back);
                                            cx.notify();
                                        });
                                    }),
                            )
                        })
                        // Status dot (list view only)
                        .when(!in_settings, |el| {
                            el.child(div().size_2().rounded_full().bg(dot))
                        })
                        // Title
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        // Aggregate status label (list view only)
                        .when(!in_settings, |el| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                                    .child(agg_label),
                            )
                        }),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(matches!(self.view, AppView::RepoList), |el| {
                        el.child(self.repo_list.clone())
                    })
                    .when_some(self.repo_settings.clone(), |el, settings| {
                        el.child(settings)
                    }),
            )
    }
}
