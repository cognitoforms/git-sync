import { useEffect, useState } from "react";
import { ArrowsClockwise, GearSix } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { syncNow, formatLastSync } from "@/api";
import type { AppStatus, DesktopConfig } from "@/types";
import StatusDot from "./StatusDot";

interface Props {
  config: DesktopConfig;
  status: AppStatus;
  onOpenSettings: (idx: number | null) => void;
}

export default function RepoListView({
  config,
  status,
  onOpenSettings,
}: Props) {
  const [, setTick] = useState(0);

  // Re-render every second to keep relative time labels fresh.
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(id);
  }, []);

  const repos = config.repositories;

  return (
    <div className="flex flex-col h-full">
      {/* Repository table */}
      <div className="flex-1 overflow-y-auto">
        {repos.length === 0 ? (
          <div className="flex items-center justify-center h-full text-sm text-muted-foreground px-6 text-center">
            No repositories configured. Add one to get started.
          </div>
        ) : (
          <table className="w-full text-sm border-collapse table-fixed">
            <colgroup>
              <col className="w-auto" />
              <col className="w-36" />
              <col className="w-32 hidden md:table-column" />
              <col className="w-36 hidden sm:table-column" />
              <col className="w-20" />
            </colgroup>
            <thead>
              <tr className="bg-muted/50 border-b border-border text-muted-foreground uppercase tracking-wide text-[11px]">
                <th className="text-left px-3 py-2 font-medium">Repository</th>
                <th className="text-left px-3 py-2 font-medium">Sync State</th>
                <th className="text-left px-3 py-2 font-medium hidden md:table-cell">Repo State</th>
                <th className="text-left px-3 py-2 font-medium hidden sm:table-cell">Last Sync</th>
                <th className="px-3 py-2" />
              </tr>
            </thead>
            <tbody>
              {repos.map((repo, idx) => {
                const st = status.repos[idx];
                return (
                  <tr
                    key={idx}
                    className="border-b border-border/50 hover:bg-muted/30 transition-colors"
                  >
                    <td className="px-3 py-2.5 align-middle">
                      <div className="font-medium text-foreground">
                        {repo.name ||
                          repo.repo_path.split(/[\\/]/).pop() ||
                          repo.repo_path}
                      </div>
                      <div className="text-[11px] text-muted-foreground mt-0.5 font-mono">
                        {repo.repo_path}
                      </div>
                    </td>
                    <td className="px-3 py-2.5 align-middle">
                      {st ? (
                        <div className="flex items-center gap-1.5">
                          <StatusDot
                            id={st.sync_state_id}
                            syncing={st.is_syncing}
                          />
                          <span className="text-foreground">
                            {st.is_syncing ? "Syncing…" : st.sync_state_label}
                          </span>
                        </div>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </td>
                    <td className="px-3 py-2.5 align-middle text-foreground hidden md:table-cell">
                      {st?.repo_state_label ?? (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </td>
                    <td className="px-3 py-2.5 align-middle text-muted-foreground whitespace-nowrap hidden sm:table-cell">
                      {st ? formatLastSync(st.last_sync_time) : "—"}
                    </td>
                    <td className="px-3 py-2.5 align-middle whitespace-nowrap">
                      <div className="flex items-center gap-1">
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => syncNow(idx)}
                          title="Sync now"
                        >
                          <ArrowsClockwise weight="bold" className={st?.is_syncing ? "animate-spin" : undefined} />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => onOpenSettings(idx)}
                          title="Settings"
                        >
                          <GearSix weight="bold" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      {/* Footer */}
      <div className="px-3 py-2.5 border-t border-border">
        <Button size="sm" onClick={() => onOpenSettings(null)}>
          + Add Repository
        </Button>
      </div>
    </div>
  );
}
