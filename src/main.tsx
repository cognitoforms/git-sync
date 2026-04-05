import "./index.css";
import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import DiffViewerApp from "./DiffViewerApp";
import { ThemeProvider } from "./components/ThemeProvider";
import { Toaster } from "./components/ui/sonner";

const queryClient = new QueryClient();
const currentWindow = getCurrentWindow();
const isDiffViewerWindow = currentWindow.label.startsWith("diff-viewer-");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<ThemeProvider>
             {isDiffViewerWindow ? <DiffViewerApp /> : <App />}
				<Toaster position="bottom-center" offset={{ bottom: 60 }} />
			</ThemeProvider>
		</QueryClientProvider>
	</React.StrictMode>,
);
