# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands

```bash
# Check compilation
cargo check -p git-sync-desktop

# Lint
cargo clippy -p git-sync-desktop

# Build
cargo build -p git-sync-desktop

# Run
cargo run -p git-sync-desktop

# Tests (minimal coverage currently — only git-sync-lib has a placeholder test)
cargo test
```

## Architecture

### Workspace

Two crates:
- `crates/git-sync-lib` — placeholder shared library, not yet used meaningfully
- `crates/git-sync-desktop` — the desktop app (GPUI UI + system tray + background sync worker)

All git sync logic lives in the external crate **`git-sync-rs`** (`RepositorySynchronizer`, `WatchManager`, `SyncState`, `RepositoryState`). This crate is a UI/orchestration wrapper around it.

### Threading model

| Thread | Role |
|---|---|
| Main (GPUI) | UI rendering, tray event polling, status push every 500 ms |
| Background (std + tokio) | `run_background()` in `worker.rs`; runs a single-threaded tokio runtime |

**Command channel** (`tokio::sync::mpsc::UnboundedSender<BgCmd>`): main → worker for `SyncNow` and `Reconfigure`.

**Status bridge** (`Arc<Mutex<AppStatus>>`): worker writes, GPUI poll loop snapshots every 500 ms and calls `AppState::update_status()` which triggers `cx.notify()` to re-render subscribed views.

### GPUI entity/view pattern

- `AppState` is the single shared `Entity<AppState>` created in `main.rs`.
- Every view (`AppWindow`, `StatusTab`, `SettingsTab`, `FileBrowserField`) subscribes via `cx.observe(&state, |_, _, cx| cx.notify())` and reads state in `render()`.
- `AppState::update_status`, `sync_now`, and `save_and_reconfigure` are the only mutating methods.

### File browser threading quirk

`rfd::FileDialog` (sync) must run on a **separate OS thread**, not GPUI's main thread, to avoid a Win32 COM/message-loop conflict that causes a `STATUS_STACK_BUFFER_OVERRUN` panic. `FileBrowserField` handles this by writing the result into `Arc<Mutex<Option<String>>>` and applying it in the next `render()` call where `&mut Window` is available.

### Key files

| File | Purpose |
|---|---|
| `src/main.rs` | App init, tray setup, 50 ms GPUI poll loop |
| `src/worker.rs` | Background sync loop; `tokio::select!` on poll + file-watcher tasks |
| `src/state.rs` | `AppState` entity — the shared model between all UI views |
| `src/config.rs` | `DesktopConfig` struct + TOML load/save; config stored at `~/.config/Cognito Forms/Git Sync/desktop.toml` |
| `src/status.rs` | `AppStatus` struct + label/ID helpers |
| `src/ui/file_browser_field.rs` | Reusable Input + Browse button component |

### GPUI API notes (version 0.2.2 + gpui-component 0.5.1)

- `cx.observe` closure takes **3 args**: `|_this, _entity, cx|`
- `Tab::new()` takes **no arguments**
- `.when()` on `Div` requires `use gpui::prelude::FluentBuilder as _`
- `TitleBar::title_bar_options()` is passed to `WindowOptions::titlebar` for the native title bar / drag region
