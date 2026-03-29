use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    input::{Input, InputState},
    v_flex,
};

use super::file_browser_field::FileBrowserField;
use crate::config::DesktopConfig;
use crate::state::AppState;

pub struct SettingsTab {
    state: Entity<AppState>,
    repo_path: Entity<FileBrowserField>,
    remote: Entity<InputState>,
    branch: Entity<InputState>,
    interval_secs: Entity<InputState>,
    commit_message: Entity<InputState>,
    sync_new_files: bool,
    skip_hooks: bool,
    conflict_branch: bool,
    _sub: Subscription,
}

impl SettingsTab {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cfg = state.read(cx).config.clone();
        let sub = cx.observe(&state, |_, _entity, cx| cx.notify());

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
        let interval_secs =
            cx.new(|cx| InputState::new(window, cx).default_value(cfg.interval_secs.to_string()));
        let commit_message = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(cfg.commit_message.clone())
                .placeholder("changes from {hostname} on {timestamp}")
        });

        Self {
            state,
            repo_path,
            remote,
            branch,
            interval_secs,
            commit_message,
            sync_new_files: cfg.sync_new_files,
            skip_hooks: cfg.skip_hooks,
            conflict_branch: cfg.conflict_branch,
            _sub: sub,
        }
    }

    fn collect_config(&self, cx: &App) -> DesktopConfig {
        DesktopConfig {
            repo_path: self.repo_path.read(cx).value(cx),
            remote: {
                let r = self.remote.read(cx).value().to_string();
                if r.is_empty() {
                    "origin".to_string()
                } else {
                    r
                }
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

impl Render for SettingsTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();

        v_flex()
            .gap_3()
            .p_4()
            .size_full()
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
                        let new_cfg = this.collect_config(cx);
                        state.update(cx, |s, _| s.save_and_reconfigure(new_cfg));
                    })),
            )
    }
}
