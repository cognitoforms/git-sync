use git_sync_lib::{RepositoryState, SyncState};

#[derive(Clone, Debug)]
pub struct RepoStatus {
    #[allow(dead_code)]
    pub repo_path: String,
    pub sync_state_label: String,
    pub sync_state_id: String,
    pub repo_state_label: String,
    pub is_syncing: bool,
    pub error: Option<String>,
    pub last_sync_time: Option<chrono::DateTime<chrono::Local>>,
}

impl RepoStatus {
    pub fn new_unconfigured(repo_path: String) -> Self {
        Self {
            repo_path,
            sync_state_label: "No repository configured".to_string(),
            sync_state_id: "unknown".to_string(),
            repo_state_label: "—".to_string(),
            is_syncing: false,
            error: None,
            last_sync_time: None,
        }
    }

    pub fn new_loading(repo_path: String) -> Self {
        Self {
            repo_path,
            sync_state_label: "Loading…".to_string(),
            sync_state_id: "unknown".to_string(),
            repo_state_label: "Unknown".to_string(),
            is_syncing: false,
            error: None,
            last_sync_time: None,
        }
    }
}

#[derive(Clone, Default)]
pub struct AppStatus {
    pub repos: Vec<RepoStatus>,
}

pub fn sync_state_label(s: &SyncState) -> String {
    match s {
        SyncState::Equal => "Up to date".to_string(),
        SyncState::Ahead(n) => format!("Ahead by {} commit(s) — push pending", n),
        SyncState::Behind(n) => format!("Behind by {} commit(s) — pull pending", n),
        SyncState::Diverged { ahead, behind } => {
            format!("Diverged: {} ahead, {} behind", ahead, behind)
        }
        SyncState::NoUpstream => "No upstream branch configured".to_string(),
    }
}

pub fn sync_state_id(s: &SyncState) -> &'static str {
    match s {
        SyncState::Equal => "equal",
        SyncState::Ahead(_) => "ahead",
        SyncState::Behind(_) => "behind",
        SyncState::Diverged { .. } => "diverged",
        SyncState::NoUpstream => "no-upstream",
    }
}

pub fn repo_state_label(s: &RepositoryState) -> &'static str {
    match s {
        RepositoryState::Clean => "Clean",
        RepositoryState::Dirty => "Dirty (uncommitted changes)",
        RepositoryState::Rebasing => "Rebase in progress",
        RepositoryState::Merging => "Merge in progress",
        RepositoryState::CherryPicking => "Cherry-pick in progress",
        RepositoryState::Bisecting => "Bisecting",
        RepositoryState::ApplyingPatches => "Applying patches",
        RepositoryState::Reverting => "Reverting",
        RepositoryState::DetachedHead => "Detached HEAD",
    }
}

pub fn format_last_sync(t: Option<chrono::DateTime<chrono::Local>>) -> String {
    let Some(dt) = t else {
        return "Never".to_string();
    };
    let secs = (chrono::Local::now() - dt).num_seconds().max(0) as u64;
    if secs < 5 {
        "Just now".to_string()
    } else if secs < 60 {
        format!("{} seconds ago", secs)
    } else if secs < 3600 {
        format!("{} minute(s) ago", secs / 60)
    } else {
        format!("{} hour(s) ago", secs / 3600)
    }
}
