import { useState, useEffect, useCallback } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type UpdaterStatus = "idle" | "checking" | "installing";

export interface UpdaterState {
	update: Update | null;
	status: UpdaterStatus;
	install: () => Promise<void>;
}

export function useUpdater(): UpdaterState {
	const [update, setUpdate] = useState<Update | null>(null);
	const [status, setStatus] = useState<UpdaterStatus>("checking");

	useEffect(() => {
		check()
			.then((u) => setUpdate(u ?? null))
			.catch((e) => console.warn("[updater] update check failed:", e))
			.finally(() => setStatus("idle"));
	}, []);

	const install = useCallback(async () => {
		if (!update || status === "installing") return;
		setStatus("installing");
		try {
			await update.downloadAndInstall();
			await relaunch();
		} catch (e) {
			console.warn("[updater] update install failed:", e);
			setStatus("idle");
			throw e;
		}
	}, [update, status]);

	return { update, status, install };
}
