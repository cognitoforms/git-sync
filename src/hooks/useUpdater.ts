import { useState, useEffect, useCallback } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type UpdaterStatus = "idle" | "checking" | "installing";

export interface UpdaterState {
	update: Update | null;
	status: UpdaterStatus;
	dismissed: boolean;
	install: () => Promise<void>;
	dismiss: () => void;
}

export function useUpdater(): UpdaterState {
	const [update, setUpdate] = useState<Update | null>(null);
	const [status, setStatus] = useState<UpdaterStatus>("checking");
	const [dismissed, setDismissed] = useState(false);

	useEffect(() => {
		const run = () =>
			check()
				.then((u) => setUpdate(u ?? null))
				.catch((e) => console.warn("[updater] update check failed:", e))
				.finally(() => setStatus("idle"));

		run();
		const id = setInterval(run, 30 * 60 * 1000);
		return () => clearInterval(id);
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

	const dismiss = useCallback(() => setDismissed(true), []);

	return { update, status, dismissed, install, dismiss };
}
