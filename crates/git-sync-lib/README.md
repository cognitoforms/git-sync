# git-sync-lib

A vendored fork of [git-sync-rs](https://github.com/colonelpanic8/git-sync-rs) by
[Ivan Malison](https://github.com/colonelpanic8), adapted for use as the sync engine
in this desktop application.

## Changes from upstream (git-sync-rs 0.7.7)

- **Async-runtime agnostic**: tokio has been removed from `[dependencies]` (kept in
  `[dev-dependencies]` for tests only). The watch event loop now uses `async-channel`,
  `futures-timer`, and `futures` primitives that work with any executor.
- **Tray icon feature removed**: the `tray` feature flag and all `#[cfg(feature = "tray")]`
  code have been stripped out.
- **Enhanced `WatchManager` public API**: a [`WatchStatusHandle`] can be obtained before
  calling `watch()`. It exposes all state the UI needs — `is_syncing`, `is_suspended`,
  `last_successful_sync`, `last_error`, `last_sync_state`, and `last_repo_state` — without
  requiring a separate `RepositorySynchronizer` for polling.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Original work copyright (c) Ivan Malison <IvanMalison@gmail.com>.
