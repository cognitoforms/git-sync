import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppStatus, DesktopConfig } from "./types";

export const getConfig = () => invoke<DesktopConfig>("get_config");
export const getStatus = () => invoke<AppStatus>("get_status");
export const setConfig = (config: DesktopConfig) =>
	invoke<void>("set_config", { config });
export const syncNow = (index: number) => invoke<void>("sync_now", { index });
export const validateRepoPath = (path: string) =>
	invoke<boolean>("validate_repo_path", { path });

export const pickFolder = () =>
	open({ directory: true, multiple: false }) as Promise<string | null>;

export const onStatusUpdate = (cb: (s: AppStatus) => void) =>
	listen<AppStatus>("status-update", (e) => cb(e.payload));

export function formatLastSync(lastSyncTime: string | null): string {
	if (!lastSyncTime) return "Never";
	const dt = new Date(lastSyncTime);
	const secs = Math.floor((Date.now() - dt.getTime()) / 1000);
	if (secs < 5) return "Just now";
	if (secs < 60) return `${secs} seconds ago`;
	if (secs < 3600) return `${Math.floor(secs / 60)} minute(s) ago`;
	return `${Math.floor(secs / 3600)} hour(s) ago`;
}
