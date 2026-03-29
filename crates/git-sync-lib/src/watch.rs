mod event_filter;

use self::event_filter::EventFilter;
use crate::error::{Result, SyncError};
use crate::sync::{RepositoryState, RepositorySynchronizer, SyncConfig, SyncState};
use async_channel::{Receiver, Sender};
use futures::channel::oneshot;
use futures::future::{self, Either};
use futures_timer::Delay;
use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Watch mode configuration
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// How long to wait after changes before syncing (milliseconds)
    pub debounce_ms: u64,

    /// Minimum interval between syncs (milliseconds)
    pub min_interval_ms: u64,

    /// Whether to sync on startup
    pub sync_on_start: bool,

    /// Dry run mode — detect changes but don't sync
    pub dry_run: bool,

    /// Optional periodic sync interval in milliseconds.
    /// When set, sync attempts are triggered even without filesystem events.
    pub periodic_sync_interval_ms: Option<u64>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            min_interval_ms: 1000,
            sync_on_start: true,
            dry_run: false,
            periodic_sync_interval_ms: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared status state
// ---------------------------------------------------------------------------

struct WatchStatusInner {
    is_syncing: AtomicBool,
    sync_suspended: AtomicBool,
    last_successful_sync_unix_secs: AtomicI64,
    last_error: Mutex<Option<String>>,
    last_sync_state: Mutex<Option<SyncState>>,
    last_repo_state: Mutex<Option<RepositoryState>>,
}

impl WatchStatusInner {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            is_syncing: AtomicBool::new(false),
            sync_suspended: AtomicBool::new(false),
            last_successful_sync_unix_secs: AtomicI64::new(0),
            last_error: Mutex::new(None),
            last_sync_state: Mutex::new(None),
            last_repo_state: Mutex::new(None),
        })
    }
}

/// A point-in-time snapshot of all watch/sync status fields.
#[derive(Debug, Clone, Default)]
pub struct WatchStatusSnapshot {
    pub is_syncing: bool,
    pub is_suspended: bool,
    pub last_successful_sync: Option<chrono::DateTime<chrono::Local>>,
    pub last_error: Option<String>,
    /// Sync state (local vs remote divergence) as of the last completed sync.
    pub last_sync_state: Option<SyncState>,
    /// Repository state (clean, dirty, rebasing, …) as of the last completed sync.
    pub last_repo_state: Option<RepositoryState>,
}

/// A cloneable, shareable handle to the [`WatchManager`]'s current status.
///
/// Obtain one before calling [`WatchManager::watch()`] via
/// [`WatchManager::status_handle()`]. The handle can be cloned and passed to UI
/// threads to observe sync state without requiring access to the manager itself.
#[derive(Clone)]
pub struct WatchStatusHandle {
    inner: Arc<WatchStatusInner>,
}

impl WatchStatusHandle {
    /// Returns a point-in-time snapshot of all status fields.
    pub fn snapshot(&self) -> WatchStatusSnapshot {
        WatchStatusSnapshot {
            is_syncing: self.is_syncing(),
            is_suspended: self.is_suspended(),
            last_successful_sync: self.last_successful_sync(),
            last_error: self.last_error(),
            last_sync_state: self.last_sync_state(),
            last_repo_state: self.last_repo_state(),
        }
    }

    /// Whether a sync is currently in progress.
    pub fn is_syncing(&self) -> bool {
        self.inner.is_syncing.load(Ordering::Acquire)
    }

    /// Whether sync is currently suspended.
    pub fn is_suspended(&self) -> bool {
        self.inner.sync_suspended.load(Ordering::Acquire)
    }

    /// Timestamp of the last successful sync, if any.
    pub fn last_successful_sync(&self) -> Option<chrono::DateTime<chrono::Local>> {
        let unix_secs = self
            .inner
            .last_successful_sync_unix_secs
            .load(Ordering::Acquire);
        if unix_secs <= 0 {
            return None;
        }
        use chrono::TimeZone;
        chrono::Local.timestamp_opt(unix_secs, 0).single()
    }

    /// Error message from the last failed sync, if any.
    pub fn last_error(&self) -> Option<String> {
        self.inner.last_error.lock().unwrap().clone()
    }

    /// Sync state (local vs remote divergence) as of the last completed sync.
    pub fn last_sync_state(&self) -> Option<SyncState> {
        self.inner.last_sync_state.lock().unwrap().clone()
    }

    /// Repository state (clean, dirty, rebasing, …) as of the last completed sync.
    pub fn last_repo_state(&self) -> Option<RepositoryState> {
        self.inner.last_repo_state.lock().unwrap().clone()
    }

    /// Suspend all sync activity. Syncs already in progress are not interrupted.
    pub fn suspend(&self) {
        self.inner.sync_suspended.store(true, Ordering::Release);
    }

    /// Resume sync activity.
    pub fn resume(&self) {
        self.inner.sync_suspended.store(false, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// File event handler (runs inside the notify callback)
// ---------------------------------------------------------------------------

struct FileEventHandler {
    repo_path: PathBuf,
    tx: Sender<Event>,
}

impl FileEventHandler {
    fn new(repo_path: PathBuf, tx: Sender<Event>) -> Self {
        Self { repo_path, tx }
    }

    fn handle_event(&self, res: std::result::Result<Event, notify::Error>) {
        let event = match res {
            Ok(event) => event,
            Err(e) => {
                error!("Watch error: {}", e);
                return;
            }
        };

        debug!("Raw file event received: {:?}", event);

        if !self.should_process_event(&event) {
            return;
        }

        debug!("Event is relevant, sending to channel");
        if let Err(e) = self.tx.try_send(event) {
            error!("Failed to send event to channel: {}", e);
        } else {
            debug!("Event sent successfully");
        }
    }

    fn should_process_event(&self, event: &Event) -> bool {
        EventFilter::should_process_event(&self.repo_path, event)
    }
}

// ---------------------------------------------------------------------------
// WatchManager
// ---------------------------------------------------------------------------

/// Manages file system watching and automatic synchronization.
pub struct WatchManager {
    repo_path: String,
    sync_config: SyncConfig,
    watch_config: WatchConfig,
    status: Arc<WatchStatusInner>,
}

impl WatchManager {
    /// Create a new watch manager.
    pub fn new(
        repo_path: impl AsRef<Path>,
        sync_config: SyncConfig,
        watch_config: WatchConfig,
    ) -> Self {
        let path_str = repo_path.as_ref().to_string_lossy();
        let expanded = shellexpand::tilde(&path_str).to_string();

        Self {
            repo_path: expanded,
            sync_config,
            watch_config,
            status: WatchStatusInner::new(),
        }
    }

    /// Returns a cloneable handle to this manager's shared status.
    ///
    /// Call this before `watch()` to obtain a handle for the UI or other
    /// threads. The handle remains valid for the lifetime of the
    /// `WatchStatusInner` Arc (i.e. for as long as the manager or any clone
    /// of the handle is alive).
    pub fn status_handle(&self) -> WatchStatusHandle {
        WatchStatusHandle {
            inner: Arc::clone(&self.status),
        }
    }

    /// Start watching for changes.
    ///
    /// Runs until the internal watcher is dropped (i.e. when this future is
    /// cancelled), the event channel closes, or an unrecoverable error occurs.
    /// This future is runtime-agnostic and can be driven by any async executor.
    pub async fn watch(&self) -> Result<()> {
        info!("Starting watch mode for: {}", self.repo_path);

        if self.watch_config.sync_on_start {
            info!("Performing initial sync");
            self.perform_sync().await?;
        }

        let (tx, rx) = async_channel::unbounded::<Event>();

        let _watcher = self.setup_watcher(tx)?;

        info!(
            "Watching for changes (debounce: {}s)",
            self.watch_config.debounce_ms as f64 / 1000.0
        );

        self.process_events(rx).await
    }

    fn setup_watcher(&self, tx: Sender<Event>) -> Result<RecommendedWatcher> {
        let repo_path_clone = PathBuf::from(&self.repo_path);
        let handler = FileEventHandler::new(repo_path_clone, tx);

        let mut watcher = RecommendedWatcher::new(
            move |res| handler.handle_event(res),
            NotifyConfig::default(),
        )?;

        watcher.watch(Path::new(&self.repo_path), RecursiveMode::Recursive)?;

        Ok(watcher)
    }

    async fn process_events(&self, rx: Receiver<Event>) -> Result<()> {
        let mut sync_state = SyncScheduler::new(
            self.watch_config.debounce_ms,
            self.watch_config.min_interval_ms,
        );

        let tick_ms = self
            .watch_config
            .debounce_ms
            .min(self.watch_config.min_interval_ms)
            .max(50);
        let tick_duration = Duration::from_millis(tick_ms);

        if let Some(interval_ms) = self.watch_config.periodic_sync_interval_ms {
            info!(
                "Periodic sync enabled (interval: {}s)",
                interval_ms as f64 / 1000.0
            );
        }

        let periodic_duration = self
            .watch_config
            .periodic_sync_interval_ms
            .map(Duration::from_millis);

        // Track when the next periodic sync should fire.  Skip first tick to
        // mirror the original behaviour of discarding the first interval tick.
        let mut next_periodic_at: Option<Instant> =
            periodic_duration.map(|d| Instant::now() + d);

        loop {
            let now = Instant::now();

            // Compute how long to wait: minimum of tick_duration and remaining
            // time until the next periodic sync (if enabled).
            let wait = match next_periodic_at {
                Some(t) if t <= now => Duration::ZERO,
                Some(t) => tick_duration.min(t - now),
                None => tick_duration,
            };

            // Race: next file event vs. deadline.
            let recv_fut = Box::pin(rx.recv());
            let delay_fut = Box::pin(Delay::new(wait));

            match future::select(recv_fut, delay_fut).await {
                Either::Left((Ok(event), _)) => {
                    self.handle_file_event(event, &mut sync_state);
                }
                Either::Left((Err(_), _)) => {
                    // All senders dropped — watcher shut down.
                    return Ok(());
                }
                Either::Right(_) => {
                    let now = Instant::now();

                    // Trigger periodic sync when its deadline passes.
                    if next_periodic_at.map_or(false, |t| now >= t) {
                        next_periodic_at = periodic_duration.map(|d| now + d);
                        sync_state.request_sync_now();
                    }

                    self.handle_timeout(&mut sync_state).await;
                }
            }
        }
    }

    fn handle_file_event(&self, event: Event, sync_state: &mut SyncScheduler) {
        debug!("Received event from channel: {:?}", event);
        debug!("Event kind: {:?}, paths: {:?}", event.kind, event.paths);

        if EventFilter::is_relevant_change(&event) {
            info!("Relevant change detected, marking pending sync");
            sync_state.mark_file_event();
        } else {
            debug!("Event not considered relevant: {:?}", event.kind);
        }
    }

    async fn handle_timeout(&self, sync_state: &mut SyncScheduler) {
        if self.status.sync_suspended.load(Ordering::Acquire) {
            return;
        }

        if !sync_state.should_sync_now() {
            return;
        }

        if self.status.is_syncing.load(Ordering::Acquire) {
            debug!("Sync already in progress, skipping");
            return;
        }

        info!("Changes detected, triggering sync");
        let span = tracing::info_span!(
            "perform_sync_attempt",
            repo = %self.repo_path,
            branch = %self.sync_config.branch_name,
            remote = %self.sync_config.remote_name,
            dry_run = self.watch_config.dry_run
        );
        let _guard = span.enter();
        match self.perform_sync().await {
            Ok(()) => {
                debug!("perform_sync succeeded");
                sync_state.on_sync_success();
            }
            Err(e) => {
                sync_state.on_sync_failure(&e);
                self.log_sync_error(&e);
            }
        }
    }

    /// Perform one synchronization cycle.
    ///
    /// Spawns the blocking git work on a dedicated OS thread so that the async
    /// event loop is not stalled.  Compatible with any async runtime.
    async fn perform_sync(&self) -> Result<()> {
        if self.status.sync_suspended.load(Ordering::Acquire) {
            debug!("Sync is suspended, skipping sync attempt");
            return Ok(());
        }

        // Guard against concurrent syncs.
        if self.status.is_syncing.swap(true, Ordering::AcqRel) {
            debug!("Sync already in progress");
            return Ok(());
        }

        let result: Result<()> = if self.watch_config.dry_run {
            info!("DRY RUN: Would perform sync now");
            Ok(())
        } else {
            let repo_path = self.repo_path.clone();
            let sync_config = self.sync_config.clone();
            let status = Arc::clone(&self.status);

            let (tx, rx) = oneshot::channel::<Result<()>>();

            std::thread::spawn(move || {
                let result: Result<()> = (|| {
                    let mut synchronizer = RepositorySynchronizer::new_with_detected_branch(
                        &repo_path,
                        sync_config,
                    )?;
                    let sync_result = synchronizer.sync(false);

                    // Capture post-sync state for the UI (best-effort; errors discarded).
                    *status.last_sync_state.lock().unwrap() =
                        synchronizer.get_sync_state().ok();
                    *status.last_repo_state.lock().unwrap() =
                        synchronizer.get_repository_state().ok();

                    sync_result
                })();

                let _ = tx.send(result);
            });

            match rx.await {
                Ok(inner) => inner,
                Err(_) => Err(SyncError::TaskError(
                    "Sync thread disconnected".to_string(),
                )),
            }
        };

        self.status.is_syncing.store(false, Ordering::Release);

        if result.is_ok() {
            self.status
                .last_successful_sync_unix_secs
                .store(chrono::Utc::now().timestamp(), Ordering::Release);
        }

        *self.status.last_error.lock().unwrap() =
            result.as_ref().err().map(ToString::to_string);

        if let Err(ref err) = result {
            error!(error = %err, "perform_sync finished with error");
        } else {
            debug!("perform_sync finished successfully");
        }

        result
    }

    fn log_sync_error(&self, e: &SyncError) {
        match e {
            SyncError::DetachedHead => {
                error!("Sync failed: detached HEAD. Repository must be on a branch; will retry.")
            }
            SyncError::UnsafeRepositoryState { state } => error!(
                state = %state,
                "Sync failed: repository in unsafe state; will retry"
            ),
            SyncError::ManualInterventionRequired { reason } => warn!(
                reason = %reason,
                "Sync requires manual intervention; pending will remain set"
            ),
            SyncError::NoRemoteConfigured { branch } => error!(
                branch = %branch,
                "Sync failed: no remote configured for branch"
            ),
            SyncError::NetworkError(msg) => error!(
                error = %msg,
                "Network error during sync; will retry"
            ),
            SyncError::TaskError(msg) => error!(
                error = %msg,
                "Background task error during sync; will retry"
            ),
            SyncError::GitError(err) => error!(
                code = ?err.code(),
                klass = ?err.class(),
                message = %err.message(),
                "Git error during sync; will retry"
            ),
            other => error!(error = %other, "Sync failed; will retry"),
        }
    }
}

// ---------------------------------------------------------------------------
// SyncScheduler — deadline/backoff based scheduler
// ---------------------------------------------------------------------------

/// Coalesces file events via a quiet debounce window, prevents starvation
/// under continuous events with a max-batch latency, and applies per-error-class
/// retry backoff on failures.
struct SyncScheduler {
    last_sync: Instant,
    pending_sync: bool,
    immediate_requested: bool,
    min_interval: Duration,
    debounce: Duration,
    max_batch_latency: Duration,
    first_event: Option<Instant>,
    last_event: Option<Instant>,
    next_retry_at: Option<Instant>,
    retry_backoff: Duration,
}

impl SyncScheduler {
    const RETRY_BACKOFF_INITIAL: Duration = Duration::from_secs(1);
    const RETRY_BACKOFF_MAX: Duration = Duration::from_secs(60);
    const RETRY_DELAY_MANUAL: Duration = Duration::from_secs(30);
    const RETRY_DELAY_CONFIG: Duration = Duration::from_secs(60);
    const RETRY_DELAY_STATE: Duration = Duration::from_secs(5);

    fn new(debounce_ms: u64, min_interval_ms: u64) -> Self {
        let debounce = Duration::from_millis(debounce_ms);
        let min_interval = Duration::from_millis(min_interval_ms);
        let max_batch_latency = debounce
            .saturating_mul(8)
            .max(min_interval)
            .max(Duration::from_millis(500));

        Self {
            last_sync: Instant::now(),
            pending_sync: false,
            immediate_requested: false,
            min_interval,
            debounce,
            max_batch_latency,
            first_event: None,
            last_event: None,
            next_retry_at: None,
            retry_backoff: Self::RETRY_BACKOFF_INITIAL,
        }
    }

    fn mark_file_event(&mut self) {
        self.mark_file_event_at(Instant::now());
    }

    fn mark_file_event_at(&mut self, now: Instant) {
        self.pending_sync = true;
        self.immediate_requested = false;
        self.first_event.get_or_insert(now);
        self.last_event = Some(now);
    }

    fn request_sync_now(&mut self) {
        self.request_sync_now_at(Instant::now());
    }

    fn request_sync_now_at(&mut self, now: Instant) {
        self.pending_sync = true;
        self.immediate_requested = true;
        self.first_event.get_or_insert(now);
        self.last_event.get_or_insert(now);
    }

    fn should_sync_now(&self) -> bool {
        self.should_sync_at(Instant::now())
    }

    fn should_sync_at(&self, now: Instant) -> bool {
        if !self.pending_sync {
            return false;
        }

        if let Some(next_retry_at) = self.next_retry_at {
            if now < next_retry_at {
                return false;
            }
        }

        if now.duration_since(self.last_sync) < self.min_interval {
            return false;
        }

        if self.immediate_requested {
            return true;
        }

        let quiet_ready = self
            .last_event
            .map(|last| now.duration_since(last) >= self.debounce)
            .unwrap_or(false);
        if quiet_ready {
            return true;
        }

        self.first_event
            .map(|first| now.duration_since(first) >= self.max_batch_latency)
            .unwrap_or(false)
    }

    fn on_sync_success(&mut self) {
        self.on_sync_success_at(Instant::now());
    }

    fn on_sync_success_at(&mut self, now: Instant) {
        self.last_sync = now;
        self.pending_sync = false;
        self.immediate_requested = false;
        self.first_event = None;
        self.last_event = None;
        self.next_retry_at = None;
        self.retry_backoff = Self::RETRY_BACKOFF_INITIAL;
    }

    fn on_sync_failure(&mut self, error: &SyncError) {
        self.on_sync_failure_at(error, Instant::now());
    }

    fn on_sync_failure_at(&mut self, error: &SyncError, now: Instant) {
        self.last_sync = now;
        self.pending_sync = true;
        self.immediate_requested = false;

        let delay = self.retry_delay_for(error);
        self.next_retry_at = Some(now + delay);
        debug!(
            delay_s = delay.as_secs_f64(),
            error = %error,
            "Sync failure scheduled with retry backoff"
        );
    }

    fn retry_delay_for(&mut self, error: &SyncError) -> Duration {
        match error {
            SyncError::ManualInterventionRequired { .. } | SyncError::HookRejected { .. } => {
                Self::RETRY_DELAY_MANUAL
            }
            SyncError::NoRemoteConfigured { .. }
            | SyncError::RemoteBranchNotFound { .. }
            | SyncError::NotARepository { .. } => Self::RETRY_DELAY_CONFIG,
            SyncError::DetachedHead | SyncError::UnsafeRepositoryState { .. } => {
                Self::RETRY_DELAY_STATE
            }
            _ => {
                let delay = self.retry_backoff;
                self.retry_backoff = self
                    .retry_backoff
                    .saturating_mul(2)
                    .min(Self::RETRY_BACKOFF_MAX);
                delay
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Free function convenience wrapper
// ---------------------------------------------------------------------------

/// Run watch mode with periodic sync.
pub async fn watch_with_periodic_sync(
    repo_path: impl AsRef<Path>,
    sync_config: SyncConfig,
    mut watch_config: WatchConfig,
    sync_interval_ms: Option<u64>,
) -> Result<()> {
    watch_config.periodic_sync_interval_ms = sync_interval_ms;
    let manager = WatchManager::new(repo_path, sync_config, watch_config);
    manager.watch().await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod scheduler_tests {
    use super::SyncScheduler;
    use crate::error::SyncError;
    use std::time::{Duration, Instant};

    #[test]
    fn scheduler_waits_for_quiet_period_before_syncing() {
        let mut scheduler = SyncScheduler::new(200, 100);
        let base = Instant::now();
        scheduler.last_sync = base;
        scheduler.mark_file_event_at(base);

        assert!(!scheduler.should_sync_at(base));
        assert!(!scheduler.should_sync_at(base + Duration::from_millis(120)));
        assert!(scheduler.should_sync_at(base + Duration::from_millis(220)));
    }

    #[test]
    fn scheduler_uses_max_batch_latency_to_prevent_starvation() {
        let mut scheduler = SyncScheduler::new(500, 100);
        let base = Instant::now();
        scheduler.last_sync = base;
        scheduler.mark_file_event_at(base);

        for i in 1..40 {
            let t = base + Duration::from_millis(100 * i);
            scheduler.mark_file_event_at(t);
            assert!(
                !scheduler.should_sync_at(t),
                "Scheduler should still wait before max-batch threshold"
            );
        }

        let ready_at = base + Duration::from_millis(4000);
        scheduler.mark_file_event_at(ready_at);
        assert!(
            scheduler.should_sync_at(ready_at),
            "Scheduler should fire at max-batch latency under continuous events"
        );
    }

    #[test]
    fn scheduler_applies_retry_backoff_and_resets_on_success() {
        let mut scheduler = SyncScheduler::new(0, 0);
        let base = Instant::now();
        scheduler.last_sync = base;
        scheduler.mark_file_event_at(base);
        assert!(scheduler.should_sync_at(base));

        scheduler.on_sync_failure_at(&SyncError::NetworkError("transient".to_string()), base);
        assert!(!scheduler.should_sync_at(base + Duration::from_millis(999)));
        assert!(scheduler.should_sync_at(base + Duration::from_millis(1000)));

        let second = base + Duration::from_millis(1000);
        scheduler.on_sync_failure_at(&SyncError::NetworkError("transient".to_string()), second);
        assert!(!scheduler.should_sync_at(second + Duration::from_secs(1)));
        assert!(scheduler.should_sync_at(second + Duration::from_secs(2)));

        scheduler.on_sync_success_at(second + Duration::from_secs(2));
        let next = second + Duration::from_secs(2);
        scheduler.mark_file_event_at(next);
        assert!(scheduler.should_sync_at(next));
    }

    #[test]
    fn scheduler_uses_longer_retry_for_manual_intervention_errors() {
        let mut scheduler = SyncScheduler::new(0, 0);
        let base = Instant::now();
        scheduler.last_sync = base;
        scheduler.mark_file_event_at(base);
        assert!(scheduler.should_sync_at(base));

        scheduler.on_sync_failure_at(
            &SyncError::ManualInterventionRequired {
                reason: "conflict".to_string(),
            },
            base,
        );
        assert!(!scheduler.should_sync_at(base + Duration::from_secs(29)));
        assert!(scheduler.should_sync_at(base + Duration::from_secs(30)));
    }

    #[test]
    fn request_sync_now_bypasses_debounce_but_respects_min_interval() {
        let mut scheduler = SyncScheduler::new(10_000, 500);
        let base = Instant::now();
        scheduler.last_sync = base;

        scheduler.request_sync_now_at(base + Duration::from_millis(100));
        assert!(!scheduler.should_sync_at(base + Duration::from_millis(499)));
        assert!(scheduler.should_sync_at(base + Duration::from_millis(500)));
    }

    #[test]
    fn request_sync_now_does_not_bypass_retry_backoff() {
        let mut scheduler = SyncScheduler::new(0, 0);
        let base = Instant::now();
        scheduler.last_sync = base;
        scheduler.mark_file_event_at(base);
        assert!(scheduler.should_sync_at(base));

        scheduler.on_sync_failure_at(&SyncError::NetworkError("transient".to_string()), base);
        scheduler.request_sync_now_at(base + Duration::from_millis(100));
        assert!(!scheduler.should_sync_at(base + Duration::from_millis(999)));
        assert!(scheduler.should_sync_at(base + Duration::from_millis(1000)));
    }
}
