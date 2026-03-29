# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands

### git-sync-desktop (Tauri + React)
```bash
# Rust backend only
cargo check -p git-sync-tauri

# Full dev (starts Vite + Tauri together)
cd crates/git-sync-desktop && pnpm tauri dev

# Production build
cd crates/git-sync-desktop && pnpm tauri build

# Frontend typecheck
cd crates/git-sync-desktop && pnpm exec tsc --noEmit
```

### Workspace
```bash
# Tests (minimal coverage — only git-sync-lib has a placeholder test)
cargo test
```

---

## Architecture

### Workspace

Two crates:
- `crates/git-sync-lib` — vendored fork of git-sync-rs; all git sync logic (`WatchManager`, `RepositorySynchronizer`, `SyncState`, `RepositoryState`)
- `crates/git-sync-desktop/src-tauri` — desktop app using Tauri v2 + React frontend (crate name: `git-sync-tauri`)

Config file: `~/.config/Cognito Forms/Git Sync/desktop.toml`.

---

## git-sync-desktop (Tauri v2 + React)

### Structure

```
crates/git-sync-desktop/
├── src-tauri/          ← Rust backend (workspace crate: git-sync-tauri)
│   ├── src/
│   │   ├── lib.rs          ← Tauri setup, tray, worker spawn, status forwarder
│   │   ├── main.rs         ← Entry point
│   │   ├── commands.rs     ← #[tauri::command] handlers
│   │   ├── worker.rs       ← Background sync loop
│   │   ├── config.rs       ← DesktopConfig + TOML load/save
│   │   ├── status.rs       ← AppStatus/RepoStatus with Serialize
│   │   └── log_layer.rs    ← Logging setup
│   ├── tauri.conf.json
│   └── capabilities/default.json
├── src/                ← React + TypeScript frontend (Vite)
│   ├── main.tsx        ← Entry; wraps app in ThemeProvider
│   ├── App.tsx         ← Root; aggregates status, owns navigation state
│   ├── api.ts          ← invoke()/listen() wrappers + formatLastSync
│   ├── types.ts        ← TypeScript mirrors of Rust IPC structs
│   ├── index.css       ← Tailwind v4 + shadcn CSS variables (neutral theme)
│   ├── hooks/
│   │   ├── useIsFullscreen.ts  ← Detects macOS fullscreen state
│   │   └── utils.ts
│   ├── components/
│   │   ├── TitleBar.tsx            ← Custom title bar: drag region, theme toggle, window controls
│   │   ├── ThemeProvider.tsx       ← Light/dark/system theme context + localStorage persistence
│   │   ├── RepoListView.tsx        ← Repository table with live status
│   │   ├── RepoSettingsView.tsx    ← Per-repo config form
│   │   ├── RepoDetailSidebar.tsx   ← Log viewer sidebar
│   │   ├── AboutModal.tsx          ← Modal dialog
│   │   ├── RepoStatusBadge.tsx     ← Status badge component
│   │   └── StatusDot.tsx           ← Colored dot by sync state ID
│   └── components/ui/
│       └── button.tsx, checkbox.tsx, field.tsx, input.tsx, label.tsx, separator.tsx, sonner.tsx
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

Worker pattern: one task per repo, auto-respawns after 2 s on error.

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
