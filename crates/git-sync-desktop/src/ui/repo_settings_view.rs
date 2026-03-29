use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    input::{Input, InputState},
    v_flex,
};

use super::file_browser_field::FileBrowserField;
use super::nav_state::{NavRequest, NavState};
use crate::config::RepoConfig;
use crate::state::AppState;

pub struct RepoSettingsView {
    state: Entity<AppState>,
    nav: Entity<NavState>,
    idx: Option<usize>,
    name: Entity<InputState>,
    repo_path: Entity<FileBrowserField>,
    remote: Entity<InputState>,
    branch: Entity<InputState>,
    interval_secs: Entity<InputState>,
    commit_message: Entity<InputState>,
    sync_new_files: bool,
    skip_hooks: bool,
    conflict_branch: bool,
}

impl RepoSettingsView {
    pub fn new(
        state: Entity<AppState>,
        nav: Entity<NavState>,
        idx: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let cfg = idx
            .and_then(|i| state.read(cx).config.repositories.get(i).cloned())
            .unwrap_or_default();

        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(cfg.name.clone())
                .placeholder("(auto-derived from path)")
        });
        let repo_path = cx.new(|cx| {
            FileBrowserField::new(window, cx, cfg.repo_path.clone(), "/path/to/your/repo")
        });
        let remote = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(cfg.remote.clone())
                .placeholder("origin")
        });
        let branch = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(cfg.branch.clone())
                .placeholder("(auto-detect from HEAD)")
        });
        let interval_secs = cx.new(|cx| {
            InputState::new(window, cx).default_value(cfg.interval_secs.to_string())
        });
        let commit_message = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(cfg.commit_message.clone())
                .placeholder("changes from {hostname} on {timestamp}")
        });

        Self {
            state,
            nav,
            idx,
            name,
            repo_path,
            remote,
            branch,
            interval_secs,
            commit_message,
            sync_new_files: cfg.sync_new_files,
            skip_hooks: cfg.skip_hooks,
            conflict_branch: cfg.conflict_branch,
        }
    }

    fn collect_config(&self, cx: &App) -> RepoConfig {
        let repo_path = self.repo_path.read(cx).value(cx);
        let name_input = self.name.read(cx).value().to_string();
        let name = if name_input.is_empty() {
            // Derive from last path component
            std::path::Path::new(&repo_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string()
        } else {
            name_input
        };

        RepoConfig {
            name,
            repo_path,
            remote: {
                let r = self.remote.read(cx).value().to_string();
                if r.is_empty() { "origin".to_string() } else { r }
            },
            branch: self.branch.read(cx).value().to_string(),
            interval_secs: self
                .interval_secs
                .read(cx)
                .value()
                .parse::<u64>()
                .unwrap_or(60)
                .max(10),
            commit_message: self.commit_message.read(cx).value().to_string(),
            sync_new_files: self.sync_new_files,
            skip_hooks: self.skip_hooks,
            conflict_branch: self.conflict_branch,
        }
    }
}

impl Render for RepoSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state_save = self.state.clone();
        let nav_save = self.nav.clone();
        let state_del = self.state.clone();
        let nav_del = self.nav.clone();
        let idx = self.idx;

        v_flex()
            .id("repo-settings-scroll")
            .gap_3()
            .p_4()
            .size_full()
            .overflow_y_scroll()
            // Name
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Name (optional)"),
                    )
                    .child(Input::new(&self.name)),
            )
            // Repo path + Browse
            .child(self.repo_path.clone())
            // Remote
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Remote"),
                    )
                    .child(Input::new(&self.remote)),
            )
            // Branch
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Branch (auto-detect if empty)"),
                    )
                    .child(Input::new(&self.branch)),
            )
            // Interval
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Sync interval (seconds)"),
                    )
                    .child(Input::new(&self.interval_secs)),
            )
            // Checkboxes
            .child(
                Checkbox::new("sync-new-files")
                    .label("Sync new (untracked) files")
                    .checked(self.sync_new_files)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.sync_new_files = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Checkbox::new("skip-hooks")
                    .label("Skip git hooks on commit")
                    .checked(self.skip_hooks)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.skip_hooks = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Checkbox::new("conflict-branch")
                    .label("Create conflict branch on merge conflict")
                    .checked(self.conflict_branch)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.conflict_branch = *checked;
                        cx.notify();
                    })),
            )
            // Commit message
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .child("Commit message (optional)"),
                    )
                    .child(Input::new(&self.commit_message)),
            )
            // Save button
            .child(
                Button::new("save")
                    .primary()
                    .label("Save Settings")
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let cfg = this.collect_config(cx);
                        state_save.update(cx, |s, _| match idx {
                            Some(i) => s.save_repo(i, cfg),
                            None => s.add_repo(cfg),
                        });
                        nav_save.update(cx, |n, cx| { n.request = Some(NavRequest::Back); cx.notify(); });
                    })),
            )
            // Delete button (only for existing repos)
            .when(idx.is_some(), |el: gpui::Stateful<gpui::Div>| {
                el.child(
                    Button::new("delete")
                        .ghost()
                        .label("Delete Repository")
                        .cursor_pointer()
                        .on_click(cx.listener(move |_, _, _, cx| {
                            if let Some(i) = idx {
                                state_del.update(cx, |s, _| s.delete_repo(i));
                                nav_del.update(cx, |n, cx| { n.request = Some(NavRequest::Back); cx.notify(); });
                            }
                        })),
                )
            })
    }
}
