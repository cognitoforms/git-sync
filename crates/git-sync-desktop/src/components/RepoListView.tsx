import { useEffect, useRef, useState } from "react";
import { ArrowsClockwise, GearSix } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { syncNow, formatLastSync } from "@/api";
import type { AppStatus, DesktopConfig } from "@/types";
import StatusDot from "./StatusDot";
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
			const newWidth = Math.max(
				220,
				Math.min(600, startWidth + (startX - e.clientX)),
			);
			setSidebarWidth(newWidth);
		};

		const onUp = () => {
			document.body.style.cursor = "";
			document.removeEventListener("mousemove", onMove);
			document.removeEventListener("mouseup", onUp);
			localStorage.setItem(
				"git-sync-sidebar-width",
				String(sidebarWidthRef.current),
			);
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
			<div className="flex min-w-0 flex-1 flex-col">
				{/* Repository table */}
				<div className="flex-1 overflow-auto">
					{repos.length === 0 ? (
						<div className="text-muted-foreground flex h-full items-center justify-center px-6 text-center text-sm">
							No repositories configured. Add one to get started.
						</div>
					) : (
						<table className="w-full min-w-[580px] table-fixed border-collapse text-sm">
							<colgroup>
								<col className="w-auto" />
								<col className="w-36" />
								<col className="hidden w-32 md:table-column" />
								<col className="hidden w-36 sm:table-column" />
								<col className="w-20" />
							</colgroup>
							<thead>
								<tr className="bg-muted/50 border-border text-muted-foreground border-b text-[11px] tracking-wide uppercase">
									<th className="px-3 py-2 text-left font-medium">
										Repository
									</th>
									<th className="px-3 py-2 text-left font-medium">
										Sync State
									</th>
									<th className="hidden px-3 py-2 text-left font-medium md:table-cell">
										Repo State
									</th>
									<th className="hidden px-3 py-2 text-left font-medium sm:table-cell">
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
											className={`border-border/50 cursor-pointer border-b transition-colors ${
												isSelected ? "bg-accent" : "hover:bg-muted/30"
											}`}
											onClick={() =>
												setSelectedRepo((prev) => (prev === idx ? null : idx))
											}
										>
											<td className="px-3 py-2.5 align-middle">
												<div className="text-foreground font-medium">
													{repo.name ||
														repo.repo_path.split(/[\\/]/).pop() ||
														repo.repo_path}
												</div>
												<div className="text-muted-foreground mt-0.5 font-mono text-[11px]">
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
											<td className="text-foreground hidden px-3 py-2.5 align-middle md:table-cell">
												{st?.repo_state_label ?? (
													<span className="text-muted-foreground">—</span>
												)}
											</td>
											<td className="text-muted-foreground hidden px-3 py-2.5 align-middle whitespace-nowrap sm:table-cell">
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
				<div className="border-border flex border-t px-3 py-2.5">
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
