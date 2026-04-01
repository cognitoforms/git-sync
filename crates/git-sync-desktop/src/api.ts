export { commands, events } from "./bindings";

export function formatLastSync(lastSyncTime: string | null): string {
	if (!lastSyncTime) return "Never";
	const dt = new Date(lastSyncTime);
	const secs = Math.floor((Date.now() - dt.getTime()) / 1000);
	if (secs < 5) return "Just now";
	if (secs < 60) return `${secs} seconds ago`;
	if (secs < 3600) return `${Math.floor(secs / 60)} minute(s) ago`;
	return `${Math.floor(secs / 3600)} hour(s) ago`;
}
