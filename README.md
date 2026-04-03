# Git Sync

A desktop app that automatically syncs your git repositories in the background — committing, pushing, and pulling on a schedule so you never lose work.

## Features

- Watch multiple repositories and sync them on a configurable interval
- Auto-commit new and changed files with a customizable commit message
- Pull remote changes and optionally create a conflict branch on merge conflicts
- File-watcher debounce for immediate sync after you save
- Runs in the system tray; minimal UI, no distractions

## Download

Pre-built binaries for Windows, macOS, and Linux are available on the [Releases](https://github.com/cognitoforms/git-sync/releases) page.

The app includes an auto-updater and will notify you when a new version is available.

## Usage

1. Launch Git Sync — it appears in your system tray.
2. Click the tray icon to open the main window.
3. Add a repository by clicking **Add Repo** and selecting the folder.
4. Configure the sync interval, remote, branch, and commit message (or leave defaults).
5. Sync runs automatically in the background. The tray icon reflects the current status.

### Configuration file

Settings are stored at:

```
~/.config/Cognito Forms/Git Sync/desktop.toml   # Linux
~/Library/Application Support/Cognito Forms/Git Sync/desktop.toml   # macOS
```

---

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/)
- [just](https://just.systems/)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS

On macOS or Linux, you can use [Nix](https://nixos.org/) to enter a shell with all prerequisites available:

```bash
nix develop
```

### Setup

```bash
pnpm install
```

### Run in development

```bash
just dev
```

This starts the Vite dev server and the Tauri app together with hot-reload.

### Build

```bash
just build
```

### Other commands

```bash
# Check formatting (Rust + frontend)
just fmt-check

# Type-check (cargo check + TypeScript)
just check

# Run tests
just test

# Regenerate src/bindings.ts (tauri-specta) without starting the app
just gen-bindings
```

### Project structure

```
src-tauri/   Rust backend (Tauri v2)
src/         React + TypeScript frontend (Vite)
crates/
  git-sync-lib/   Core sync library (fork of git-sync-rs)
```

---

## License

[MIT](LICENSE)
