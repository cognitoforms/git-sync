import { useEffect, useState } from "react";
import {
  getConfig,
  getStatus,
  onStatusUpdate,
  setConfig,
} from "./api";
import type { AppStatus, DesktopConfig, View } from "./types";
import { ERROR_LABELS } from "./components/RepoStatusBadge";

const WARNING_CATEGORIES = new Set(["conflict", "conflict_branch"]);
import TitleBar from "./components/TitleBar";
import RepoListView from "./components/RepoListView";
import RepoSettingsView from "./components/RepoSettingsView";

const EMPTY_CONFIG: DesktopConfig = { repositories: [] };
const EMPTY_STATUS: AppStatus = { repos: [] };

const STATUS_PRIORITY: Record<string, number> = {
  "error-critical": 6,
  "error-warning": 5,
  diverged: 4,
  syncing: 3,
  ahead: 2,
  behind: 2,
  equal: 1,
};

function aggregateStatus(status: AppStatus): { id: string; label: string } {
  const worst = status.repos.reduce<{ id: string; label: string } | null>(
    (acc, r) => {
      let effectiveId: string;
      let effectiveLabel: string;
      if (r.is_syncing) {
        effectiveId = "syncing";
        effectiveLabel = "Syncing…";
      } else if (r.error) {
        effectiveId = WARNING_CATEGORIES.has(r.error.category) ? "error-warning" : "error-critical";
        effectiveLabel = ERROR_LABELS[r.error.category] ?? "Sync error";
      } else {
        effectiveId = r.sync_state_id;
        effectiveLabel = r.sync_state_label;
      }
      const p = STATUS_PRIORITY[effectiveId] ?? 0;
      if (!acc || p > (STATUS_PRIORITY[acc.id] ?? 0)) {
        return { id: effectiveId, label: effectiveLabel };
      }
      return acc;
    },
    null,
  );
  return worst ?? { id: "unknown", label: "No repositories" };
}

function titleForView(view: View): string {
  if (view.kind === "settings") {
    return view.idx !== null ? "Repository Settings" : "Add Repository";
  }
  return "Git Sync";
}

export default function App() {
  const [config, setConfigState] = useState<DesktopConfig>(EMPTY_CONFIG);
  const [status, setStatus] = useState<AppStatus>(EMPTY_STATUS);
  const [view, setView] = useState<View>({ kind: "list" });

  useEffect(() => {
    getConfig().then(setConfigState).catch(console.error);
    getStatus().then(setStatus).catch(console.error);
    const unlistenPromise = onStatusUpdate(setStatus);
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, []);

  const handleSave = async (newConfig: DesktopConfig) => {
    await setConfig(newConfig);
    setConfigState(newConfig);
    setView({ kind: "list" });
  };

  const agg = aggregateStatus(status);

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      <TitleBar
        inSettings={view.kind === "settings"}
        title={titleForView(view)}
        aggStatusId={agg.id}
        aggStatusLabel={agg.label}
        onBack={() => setView({ kind: "list" })}
      />
      <div className="flex-1 overflow-hidden">
        {view.kind === "list" && (
          <RepoListView
            config={config}
            status={status}
            onOpenSettings={(idx) => setView({ kind: "settings", idx })}
          />
        )}
        {view.kind === "settings" && (
          <RepoSettingsView
            config={config}
            idx={view.idx}
            onSave={handleSave}
            onBack={() => setView({ kind: "list" })}
          />
        )}
      </div>
    </div>
  );
}
