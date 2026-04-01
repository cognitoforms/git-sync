export interface RepoConfig {
	name: string;
	repo_path: string;
	remote: string;
	branch: string;
	interval_secs: number;
	commit_message: string;
	sync_new_files: boolean;
	skip_hooks: boolean;
	conflict_branch: boolean;
	sync_on_start: boolean;
	debounce_ms: number;
}

export interface GlobalSettings {
	remote: string;
	interval_secs: number;
	commit_message: string;
	sync_new_files: boolean;
	skip_hooks: boolean;
	conflict_branch: boolean;
	sync_on_start: boolean;
	debounce_ms: number;
}

export type SyncError =
	| { category: "auth"; message: string }
	| { category: "network"; message: string }
	| { category: "conflict"; message: string }
	| { category: "conflict_branch"; branch: string; message: string }
	| { category: "config"; message: string }
	| { category: "state"; message: string }
	| { category: "unknown"; message: string };

export interface RepoStatus {
	repo_path: string;
	sync_state_label: string;
	sync_state_id: string;
	repo_state_label: string;
	is_syncing: boolean;
	error: SyncError | null;
	last_sync_time: string | null; // ISO 8601 timestamp
}

export interface DesktopConfig {
	global: GlobalSettings;
	repositories: RepoConfig[];
}

export interface AppStatus {
	repos: RepoStatus[];
}

export interface LogEntry {
	timestamp: string;
	level: "info" | "warn" | "error";
	message: string;
	repo: string | null;
}

export type View =
	| { kind: "list" }
	| { kind: "settings"; idx: number | null }
	| { kind: "global-settings" }
	| { kind: "about" };
