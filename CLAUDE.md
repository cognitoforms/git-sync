# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands

### git-sync-desktop (GPUI)
```bash
cargo check -p git-sync-desktop
cargo clippy -p git-sync-desktop
cargo build -p git-sync-desktop
cargo run -p git-sync-desktop
```

### git-sync-tauri (Tauri + React)
```bash
# Rust backend only
cargo check -p git-sync-tauri

# Full dev (starts Vite + Tauri together)
cd crates/git-sync-tauri && pnpm tauri dev

# Production build
cd crates/git-sync-tauri && pnpm tauri build

# Frontend typecheck
cd crates/git-sync-tauri && pnpm exec tsc --noEmit
```

### Workspace
```bash
# Tests (minimal coverage — only git-sync-lib has a placeholder test)
cargo test
```

---

## Architecture

### Workspace

Three crates:
- `crates/git-sync-lib` — vendored fork of git-sync-rs; all git sync logic (`WatchManager`, `RepositorySynchronizer`, `SyncState`, `RepositoryState`)
- `crates/git-sync-desktop` — desktop app using GPUI 0.2.2
- `crates/git-sync-tauri/src-tauri` — desktop app using Tauri v2 + React frontend

Both desktop apps share the same config file: `~/.config/Cognito Forms/Git Sync/desktop.toml`.

---

## git-sync-desktop (GPUI)

### Threading model

| Thread | Role |
|---|---|
| Main (GPUI) | UI rendering, tray event polling, status push every 50 ms |
| Background (std + tokio) | `run_background()` in `worker.rs`; single-threaded tokio runtime |

**Command channel** (`tokio::sync::mpsc::UnboundedSender<BgCmd>`): main → worker for `SyncNow` and `Reconfigure`.

**Status bridge** (`Arc<watch::Sender<AppStatus>>`): worker writes on change, GPUI async task wakes via `.changed()` and calls `AppState::update_status()` → `cx.notify()`.

### GPUI entity/view pattern

- `AppState` is the single shared `Entity<AppState>` created in `main.rs`.
- Every view subscribes via `cx.observe(&state, |_, _, cx| cx.notify())` and reads state in `render()`.
- `AppState::update_status`, `sync_now`, and `save_and_reconfigure` are the only mutating methods.

### File browser threading quirk

`rfd::FileDialog` must run on a **separate OS thread** to avoid a Win32 COM/message-loop conflict (`STATUS_STACK_BUFFER_OVERRUN`). Result is written into `Arc<Mutex<Option<String>>>` and applied on next `render()`.

### Key files

| File | Purpose |
|---|---|
| `src/main.rs` | App init, tray setup, GPUI event loop |
| `src/worker.rs` | Background sync loop |
| `src/state.rs` | `AppState` entity — shared model |
| `src/config.rs` | `DesktopConfig` + TOML load/save |
| `src/status.rs` | `AppStatus` struct + label/ID helpers |
| `src/ui/file_browser_field.rs` | Input + Browse button component |

### GPUI API notes (version 0.2.2 + gpui-component 0.5.1)

- `cx.observe` closure takes **3 args**: `|_this, _entity, cx|`
- `Tab::new()` takes **no arguments**
- `.when()` on `Div` requires `use gpui::prelude::FluentBuilder as _`
- `TitleBar::title_bar_options()` is passed to `WindowOptions::titlebar`

---

## git-sync-tauri (Tauri v2 + React)

### Structure

```
crates/git-sync-tauri/
├── src-tauri/          ← Rust backend (workspace crate: git-sync-tauri)
│   ├── src/
│   │   ├── lib.rs      ← Tauri setup, tray, worker spawn, status forwarder
│   │   ├── commands.rs ← #[tauri::command] handlers
│   │   ├── worker.rs   ← Background sync (identical pattern to GPUI version)
│   │   ├── config.rs   ← DesktopConfig + TOML load/save (same path as GPUI app)
│   │   └── status.rs   ← AppStatus/RepoStatus with Serialize
│   ├── tauri.conf.json
│   └── capabilities/default.json
├── src/                ← React + TypeScript frontend (Vite)
│   ├── main.tsx        ← Entry; wraps app in ThemeProvider
│   ├── App.tsx         ← Root; aggregates status, owns navigation state
│   ├── api.ts          ← invoke()/listen() wrappers + formatLastSync
│   ├── types.ts        ← TypeScript mirrors of Rust IPC structs
│   ├── index.css       ← Tailwind v4 + shadcn CSS variables (neutral theme)
│   ├── components/
│   │   ├── TitleBar.tsx        ← Custom title bar: drag region, theme toggle, window controls
│   │   ├── ThemeProvider.tsx   ← Light/dark/system theme context + localStorage persistence
│   │   ├── RepoListView.tsx    ← Repository table with live status
│   │   ├── RepoSettingsView.tsx← Per-repo config form
│   │   ├── AboutModal.tsx      ← Modal dialog
│   │   └── StatusDot.tsx       ← Colored dot by sync state ID
│   └── components/ui/
│       └── button.tsx          ← shadcn Button (radix-lyra style)
└── package.json
```

### Rust backend IPC

**Tauri commands** (called from frontend via `invoke()`):

| Command | Description |
|---|---|
| `get_config` | Returns full `DesktopConfig` |
| `get_status` | Returns current `AppStatus` snapshot |
| `set_config` | Saves config to TOML + sends `Reconfigure` to worker |
| `sync_now(index)` | Sends `SyncNow(index)` to worker |
| `pick_folder` | Opens native folder picker (tauri-plugin-dialog) |

**Tauri events** (emitted from Rust to frontend):

| Event | Payload | Description |
|---|---|---|
| `status-update` | `AppStatus` | Pushed on every worker status change |

### Threading model

| Thread | Role |
|---|---|
| Main (Tauri) | Tauri event loop; manages state, tray, window events |
| Background (std + tokio) | `run_background()` in `worker.rs`; single-threaded tokio + `LocalSet` |
| Async (Tauri runtime) | Status forwarder: watches `watch::Receiver` and emits `status-update` |

Worker pattern is identical to the GPUI version: one task per repo, auto-respawns after 2 s on error.

### Custom title bar

Window has `decorations: false`. The React `TitleBar` component provides:
- `data-tauri-drag-region` on the content div (requires `core:window:allow-start-dragging` capability)
- Theme toggle (Sun/Moon) — cycles light ↔ dark, persists to `localStorage`
- Window controls (Minus/Square/X) via `@tauri-apps/api/window`
- Close button calls `appWindow.close()` → Rust `CloseRequested` handler → hides to tray

### Dark mode

`ThemeProvider` in `src/components/ThemeProvider.tsx`:
- Reads/writes `"git-sync-theme"` key in `localStorage`
- Defaults to system preference (`prefers-color-scheme`)
- Applies/removes `.dark` class on `document.documentElement`
- Listens to OS theme changes when set to `"system"`

### Frontend tech stack

- **Vite** + **React 19** + **TypeScript**
- **Tailwind CSS v4** via `@tailwindcss/vite` plugin
- **shadcn/ui** (radix-lyra style, neutral base color, phosphor icons)
- **@phosphor-icons/react** for all icons
- Package manager: **pnpm**

### Plugins required

| Plugin | Purpose |
|---|---|
| `tauri-plugin-dialog` | Native folder picker |
| `tauri-plugin-os` | OS detection (platform info) |

### Capabilities (`capabilities/default.json`)

```json
"core:default",
"core:window:allow-start-dragging",
"dialog:allow-open",
"os:default"
```
