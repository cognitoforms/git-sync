import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

const appWindow = getCurrentWindow();

export function useIsFullscreen(): boolean {
	const [isFullscreen, setIsFullscreen] = useState(false);

	useEffect(() => {
		appWindow.isFullscreen().then(setIsFullscreen);
		const unlisten = appWindow.onResized(() => {
			appWindow.isFullscreen().then(setIsFullscreen);
		});
		return () => {
			unlisten.then((f) => f());
		};
	}, []);

	return isFullscreen;
}
