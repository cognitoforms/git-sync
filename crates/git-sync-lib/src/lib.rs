pub mod config;
pub mod error;
pub mod sync;
pub mod watch;

pub use config::{Config, ConfigLoader, DefaultConfig, RepositoryConfig};
pub use error::{Result, SyncError, SyncErrorExtra, SyncErrorSummary};
pub use sync::{
    CommandGitTransport, CommitOutcome, ConflictFileContent, FallbackState, GitTransport,
    RepositoryState, RepositorySynchronizer, ResolvedFileContent, SyncConfig, SyncState,
    UnhandledFileState, FALLBACK_BRANCH_PREFIX,
};
pub use watch::{
    watch_with_periodic_sync, WatchCmd, WatchConfig, WatchHandle, WatchManager, WatchStatusSnapshot,
};
