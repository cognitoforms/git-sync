# Start Vite dev server + Tauri app with hot-reload
dev:
    pnpm tauri dev

# Build the production app
build:
    pnpm tauri build

# Check Rust formatting
fmt-check-rust:
    cargo fmt --all -- --check

# Check frontend formatting with Prettier
fmt-check-frontend:
    pnpm format:check

# Check formatting (Rust + frontend in parallel)
[parallel]
fmt-check: fmt-check-rust fmt-check-frontend

# Run cargo check on the workspace
check-rust:
    cargo check

# TypeScript typecheck
typecheck:
    pnpm typecheck

# Check all code (Rust + frontend in parallel)
[parallel]
check: check-rust typecheck

# Run workspace tests
test:
    cargo test

# Regenerate src/bindings.ts (tauri-specta) without starting the app
gen-bindings:
    cargo test -p git-sync-tauri export_bindings

# Run all CI checks
ci: fmt-check check test
