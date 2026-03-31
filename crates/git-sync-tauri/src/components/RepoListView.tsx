import { useEffect, useRef, useState } from "react";
import { ArrowsClockwise, GearSix } from "@phosphor-icons/react";

import { Button } from "@/components/ui/button";
import { syncNow, formatLastSync } from "@/api";
import type { AppStatus, DesktopConfig } from "@/types";
import RepoStatusBadge from "./RepoStatusBadge";
import RepoDetailSidebar from "./RepoDetailSidebar";
import { Transition } from "react-transition-group";

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
  const [selectedRepo, setSelectedRepo] = useState<number | null>(null);
  const [sidebarWidth, setSidebarWidth] = useState(() => {
    const saved = localStorage.getItem("git-sync-sidebar-width");
    return saved ? Math.max(220, Math.min(600, parseInt(saved, 10))) : 320;
  });
  const sidebarNodeRef = useRef<HTMLDivElement>(null);
  // Retains the last valid selection so the sidebar content stays visible
  // during the exit transition (when selectedRepo has already been cleared).
  const lastValidIdxRef = useRef<number | null>(null);
  const sidebarWidthRef = useRef(sidebarWidth);
  sidebarWidthRef.current = sidebarWidth;

  // Re-render every second to keep relative time labels fresh.
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(id);
  }, []);

  const handleDragStart = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = sidebarWidthRef.current;
    document.body.style.cursor = "col-resize";

    const onMove = (e: MouseEvent) => {
      const newWidth = Math.max(220, Math.min(600, startWidth + (startX - e.clientX)));
      setSidebarWidth(newWidth);
    };

    const onUp = () => {
      document.body.style.cursor = "";
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      localStorage.setItem("git-sync-sidebar-width", String(sidebarWidthRef.current));
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  };

  const repos = config.repositories;

  const showSidebar =
    selectedRepo !== null && config.repositories[selectedRepo] != null;

  if (showSidebar) lastValidIdxRef.current = selectedRepo;

  const displayIdx = lastValidIdxRef.current;
  const displayConfig =
    displayIdx !== null ? (config.repositories[displayIdx] ?? null) : null;
  const displayStatus =
    displayIdx !== null ? status.repos[displayIdx] : undefined;

  return (
    <div className="flex h-full">
      <div className="flex-1 min-w-0 flex flex-col">
        {/* Repository table */}
        <div className="flex-1 overflow-auto">
          {repos.length === 0 ? (
            <div className="flex items-center justify-center h-full text-sm text-muted-foreground px-6 text-center">
              No repositories configured. Add one to get started.
            </div>
          ) : (
            <table className="w-full min-w-[580px] text-sm border-collapse table-fixed">
              <colgroup>
                <col className="w-auto" />
                <col className="w-36" />
                <col className="w-32 hidden md:table-column" />
                <col className="w-36 hidden sm:table-column" />
                <col className="w-20" />
              </colgroup>
              <thead>
                <tr className="bg-muted/50 border-b border-border text-muted-foreground uppercase tracking-wide text-[11px]">
                  <th className="text-left px-3 py-2 font-medium">
                    Repository
                  </th>
                  <th className="text-left px-3 py-2 font-medium">
                    Sync State
                  </th>
                  <th className="text-left px-3 py-2 font-medium hidden md:table-cell">
                    Repo State
                  </th>
                  <th className="text-left px-3 py-2 font-medium hidden sm:table-cell">
                    Last Sync
                  </th>
                  <th className="px-3 py-2" />
                </tr>
              </thead>
              <tbody>
                {repos.map((repo, idx) => {
                  const st = status.repos[idx];
                  const isSelected = selectedRepo === idx;
                  return (
                    <tr
                      key={idx}
                      className={`border-b border-border/50 transition-colors cursor-pointer ${
                        isSelected ? "bg-accent" : "hover:bg-muted/30"
                      }`}
                      onClick={() =>
                        setSelectedRepo((prev) => (prev === idx ? null : idx))
                      }
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
                          <RepoStatusBadge status={st} />
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
                      <td
                        className="px-3 py-2.5 align-middle whitespace-nowrap"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <div className="flex items-center gap-1">
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={() => syncNow(idx)}
                            title="Sync now"
                          >
                            <ArrowsClockwise
                              weight="bold"
                              className={
                                st?.is_syncing ? "animate-spin" : undefined
                              }
                            />
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
        <div className="px-3 py-2.5 border-t border-border flex">
          <Button
            className="ml-auto"
            size="sm"
            onClick={() => onOpenSettings(null)}
          >
            + Add Repository
          </Button>
        </div>
      </div>

      {/* Sidebar */}
      <Transition nodeRef={sidebarNodeRef} in={showSidebar} timeout={300}>
        {(state) => (
          <div
            ref={sidebarNodeRef}
            style={{
              width:
                state === "entering" || state === "entered" ? sidebarWidth : 0,
              transition:
                state === "entering" || state === "exiting"
                  ? "width 180ms ease-in-out"
                  : undefined,
              overflow: "hidden",
              flexShrink: 0,
              position: "relative",
              pointerEvents: state === "exited" ? "none" : undefined,
            }}
          >
            {/* Drag handle */}
            {state === "entered" && (
              <div
                onMouseDown={handleDragStart}
                style={{
                  position: "absolute",
                  left: 0,
                  top: 0,
                  bottom: 0,
                  width: 4,
                  cursor: "col-resize",
                  zIndex: 10,
                }}
                className="hover:bg-primary/20 transition-colors"
              />
            )}
            {displayConfig != null && displayIdx !== null && (
              <RepoDetailSidebar
                idx={displayIdx}
                config={displayConfig}
                status={displayStatus}
                onClose={() => setSelectedRepo(null)}
                onSync={() => syncNow(displayIdx)}
                onOpenSettings={() => onOpenSettings(displayIdx)}
              />
            )}
          </div>
        )}
      </Transition>
    </div>
  );
}
