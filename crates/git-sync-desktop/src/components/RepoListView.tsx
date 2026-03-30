import { useEffect, useState } from "react";
import { Question } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { syncNow, formatLastSync } from "@/api";
import type { AppStatus, DesktopConfig } from "@/types";
import StatusDot from "./StatusDot";

interface Props {
	config: DesktopConfig;
	status: AppStatus;
	onOpenSettings: (idx: number | null) => void;
	onAbout: () => void;
}

export default function RepoListView({
	config,
	status,
	onOpenSettings,
	onAbout,
}: Props) {
	const [, setTick] = useState(0);

	// Re-render every second to keep relative time labels fresh.
	useEffect(() => {
		const id = setInterval(() => setTick((t) => t + 1), 1000);
		return () => clearInterval(id);
	}, []);

	const repos = config.repositories;

	return (
		<div className="flex h-full flex-col">
			{/* Toolbar */}
			<div className="border-border flex items-center justify-end border-b px-3 py-1.5">
				<Button variant="ghost" size="icon-sm" onClick={onAbout} title="About">
					<Question weight="bold" />
				</Button>
			</div>

			{/* Repository table */}
			<div className="flex-1 overflow-y-auto">
				{repos.length === 0 ? (
					<div className="text-muted-foreground flex h-full items-center justify-center px-6 text-center text-sm">
						No repositories configured. Add one to get started.
					</div>
				) : (
					<table className="w-full border-collapse text-xs">
						<thead>
							<tr className="bg-muted/50 border-border text-muted-foreground border-b text-[11px] tracking-wide uppercase">
								<th className="px-3 py-2 text-left font-medium">Repository</th>
								<th className="px-3 py-2 text-left font-medium">Sync State</th>
								<th className="px-3 py-2 text-left font-medium">Repo State</th>
								<th className="px-3 py-2 text-left font-medium">Last Sync</th>
								<th className="px-3 py-2" />
							</tr>
						</thead>
						<tbody>
							{repos.map((repo, idx) => {
								const st = status.repos[idx];
								return (
									<tr
										key={idx}
										className="border-border/50 hover:bg-muted/30 border-b transition-colors"
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
										<td className="text-foreground px-3 py-2.5 align-middle">
											{st?.repo_state_label ?? (
												<span className="text-muted-foreground">—</span>
											)}
										</td>
										<td className="text-muted-foreground px-3 py-2.5 align-middle whitespace-nowrap">
											{st ? formatLastSync(st.last_sync_time) : "—"}
										</td>
										<td className="px-3 py-2.5 align-middle whitespace-nowrap">
											<div className="flex items-center gap-1">
												<Button
													variant="outline"
													size="xs"
													onClick={() => syncNow(idx)}
												>
													Sync now
												</Button>
												<Button
													variant="ghost"
													size="xs"
													onClick={() => onOpenSettings(idx)}
												>
													Settings
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
			<div className="border-border border-t px-3 py-2.5">
				<Button size="sm" onClick={() => onOpenSettings(null)}>
					+ Add Repository
				</Button>
			</div>
		</div>
	);
}
