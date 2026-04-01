import { useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { commands, events } from "../bindings";
import type {
	DesktopConfig,
	GlobalSettings,
	RepoConfig,
	AppStatus,
} from "../bindings";

// ── Resolved types (all fields guaranteed present) ────────────────────────────
// specta generates optional fields for structs with #[serde(default)].
// We normalize on the way in so the rest of the app can treat them as required.

export type ResolvedGlobal = Required<GlobalSettings>;
export type ResolvedRepo = Required<RepoConfig>;
export type ResolvedConfig = Omit<DesktopConfig, "global" | "repositories"> & {
	global: ResolvedGlobal;
	repositories: ResolvedRepo[];
};

const DEFAULT_GLOBAL: ResolvedGlobal = {
	remote: "origin",
	interval_secs: 60,
	commit_message: "",
	sync_new_files: true,
	skip_hooks: false,
	conflict_branch: true,
	sync_on_start: true,
	debounce_ms: 500,
};

const DEFAULT_REPO: ResolvedRepo = {
	name: "",
	repo_path: "",
	remote: "origin",
	branch: "",
	interval_secs: 60,
	sync_new_files: true,
	skip_hooks: false,
	conflict_branch: true,
	commit_message: "",
	sync_on_start: true,
	debounce_ms: 500,
};

function normalizeConfig(raw: DesktopConfig): ResolvedConfig {
	return {
		...raw,
		global: { ...DEFAULT_GLOBAL, ...raw.global },
		repositories: (raw.repositories ?? []).map((r) => ({
			...DEFAULT_REPO,
			...r,
		})),
	};
}

export const EMPTY_CONFIG: ResolvedConfig = normalizeConfig({
	global: {},
	repositories: [],
});
export const EMPTY_STATUS: AppStatus = { repos: [] };

// ── Queries ───────────────────────────────────────────────────────────────────

export function useConfig() {
	return useQuery({
		queryKey: ["config"],
		queryFn: async () => normalizeConfig(await commands.getConfig()),
		placeholderData: EMPTY_CONFIG,
	});
}

export function useStatus() {
	const queryClient = useQueryClient();

	useEffect(() => {
		const p = events.statusUpdateEvent.listen((e) => {
			queryClient.setQueryData(["status"], e.payload);
		});
		return () => {
			p.then((f) => f());
		};
	}, [queryClient]);

	return useQuery({
		queryKey: ["status"],
		queryFn: () => commands.getStatus(),
		placeholderData: EMPTY_STATUS,
	});
}

export function useSetConfig() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: async (config: DesktopConfig) => {
			const result = await commands.setConfig(config);
			if (result.status === "error") throw new Error(result.error);
		},
		onSuccess: (_, config) =>
			queryClient.setQueryData(["config"], normalizeConfig(config)),
	});
}

export function useSyncNow() {
	return useMutation({
		mutationFn: async (index: number) => {
			const result = await commands.syncNow(index);
			if (result.status === "error") throw new Error(result.error);
		},
	});
}
