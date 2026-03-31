import { useRef, useEffect, useState } from "react";
import { ArrowsClockwise, GearSix, X } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { formatLastSync, getLogHistory, onLogEntry } from "@/api";
import type { LogEntry, RepoConfig, RepoStatus } from "@/types";
import StatusDot from "./StatusDot";

const LOG_CAP = 200;

interface Props {
  idx: number;
  config: RepoConfig;
  status: RepoStatus | undefined;
  onClose: () => void;
  onSync: () => void;
  onOpenSettings: () => void;
}

export default function RepoDetailSidebar({
  config,
  status,
  onClose,
  onSync,
  onOpenSettings,
}: Props) {
  const logEndRef = useRef<HTMLDivElement>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);

  useEffect(() => {
    const repoPath = config.repo_path;

    getLogHistory()
      .then((entries) => {
        setLogs(entries.filter((e) => e.repo === repoPath).slice(-LOG_CAP));
      })
      .catch(console.error);

    const unlisten = onLogEntry((entry) => {
      if (entry.repo === repoPath) {
        setLogs((prev) => [...prev, entry].slice(-LOG_CAP));
      }
    });

    return () => {
      unlisten.then((u) => u());
    };
  }, [config.repo_path]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const name =
    config.name || config.repo_path.split(/[\\/]/).pop() || config.repo_path;

  return (
    <div className="w-full h-full bg-background border-l border-border flex flex-col [box-shadow:-4px_0_16px_rgb(0_0_0_/_0.08)]">
      {/* Header */}
      <div className="px-2 border-b border-border flex items-center gap-1.5 h-8">
        <span className="flex-1 font-medium truncate text-sm">{name}</span>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onOpenSettings}
          title="Settings"
        >
          <GearSix weight="bold" />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onSync}
          title="Sync now"
        >
          <ArrowsClockwise
            weight="bold"
            className={status?.is_syncing ? "animate-spin" : undefined}
          />
        </Button>
        <Button variant="ghost" size="icon-sm" onClick={onClose} title="Close">
          <X weight="bold" />
        </Button>
      </div>

      {/* Status */}
      <div className="p-3 border-b border-border space-y-1">
        {status ? (
          <>
            <div className="flex items-center gap-1.5 text-sm">
              <StatusDot
                id={status.sync_state_id}
                syncing={status.is_syncing}
              />
              <span>
                {status.is_syncing ? "Syncing…" : status.sync_state_label}
              </span>
            </div>
            {status.repo_state_label && (
              <div className="text-xs text-muted-foreground">
                {status.repo_state_label}
              </div>
            )}
            <div className="text-xs text-muted-foreground">
              Last sync: {formatLastSync(status.last_sync_time)}
            </div>
            {status.error && (
              <div className="text-xs text-red-600 bg-red-50 dark:bg-red-950/20 rounded p-1.5 mt-1">
                {status.error}
              </div>
            )}
          </>
        ) : (
          <div className="text-sm text-muted-foreground">Loading…</div>
        )}
      </div>

      {/* Details */}
      <div className="p-3 border-b border-border text-xs text-muted-foreground space-y-1">
        <div className="flex gap-2">
          <span className="w-14 shrink-0">Path</span>
          <span className="font-mono truncate">{config.repo_path}</span>
        </div>
        <div className="flex gap-2">
          <span className="w-14 shrink-0">Remote</span>
          <span>{config.remote}</span>
        </div>
        <div className="flex gap-2">
          <span className="w-14 shrink-0">Branch</span>
          <span>{config.branch}</span>
        </div>
        <div className="flex gap-2">
          <span className="w-14 shrink-0">Interval</span>
          <span>{config.interval_secs}s</span>
        </div>
      </div>

      {/* Log */}
      <div className="flex-1 overflow-auto p-2 font-mono text-xs">
        {logs.length === 0 ? (
          <div className="text-muted-foreground text-center py-4">
            No log entries yet
          </div>
        ) : (
          logs.map((entry, i) => (
            <div
              key={i}
              className="flex gap-1.5 mb-0.5 leading-relaxed whitespace-nowrap"
            >
              <span className="text-muted-foreground shrink-0">
                {new Date(entry.timestamp).toLocaleTimeString()}
              </span>
              <span
                className={`shrink-0 ${
                  entry.level === "error"
                    ? "text-red-600"
                    : entry.level === "warn"
                      ? "text-amber-600"
                      : "text-muted-foreground"
                }`}
              >
                {entry.level}
              </span>
              <span className="text-foreground">{entry.message}</span>
            </div>
          ))
        )}
        <div ref={logEndRef} />
      </div>
    </div>
  );
}
