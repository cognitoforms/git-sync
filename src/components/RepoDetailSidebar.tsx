import { useRef, useEffect, useState } from "react";
import { ArrowsClockwise, GearSix, X } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { formatLastSync, commands, events } from "@/api";
import type { FrontendLogEntry, RepoStatus } from "@/bindings";
import type { ResolvedRepo } from "@/hooks/queries";
import RepoStatusBadge, { ERROR_LABELS } from "./RepoStatusBadge";
import ConflictPanel from "./ConflictPanel";

const ERROR_HINTS: Record<string, string> = {
	auth: "Check your SSH keys or repository credentials.",
	network: "Check your network connection and try again.",
	config: "Review the repository settings (remote, branch).",
	state:
		"The repository is in an intermediate Git state. It may resolve automatically.",
};

const LOG_CAP = 200;

interface Props {
	idx: number;
	config: ResolvedRepo;
	status: RepoStatus | undefined;
	onClose: () => void;
	onSync: () => void;
	onOpenSettings: () => void;
}

export default function RepoDetailSidebar({
	idx,
	config,
	status,
	onClose,
	onSync,
	onOpenSettings,
}: Props) {
	const logEndRef = useRef<HTMLDivElement>(null);
	const [logs, setLogs] = useState<FrontendLogEntry[]>([]);

	// Load history once per repo path, then append live entries.
	useEffect(() => {
		setLogs([]);
		commands
			.getLogHistory(config.repo_path)
			.then((entries) => setLogs(entries.slice(-LOG_CAP)))
			.catch(console.error);
	}, [config.repo_path]);

	// Subscribe to live log entries.
	useEffect(() => {
		const p = events.logEntryEvent.listen((e) => {
			const entry = e.payload;
			if (entry.repo === config.repo_path) {
				setLogs((prev) => [...prev, entry].slice(-LOG_CAP));
			}
		});
		return () => {
			p.then((f) => f());
		};
	}, [config.repo_path]);

	useEffect(() => {
		logEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [logs]);

	const name =
		config.name || config.repo_path.split(/[\\/]/).pop() || config.repo_path;

	const isConflictCategory =
		status?.error?.category === "conflict" ||
		status?.error?.category === "conflict_branch";

	return (
		<div className="bg-background border-border flex h-full w-full flex-col border-l [box-shadow:-4px_0_16px_rgb(0_0_0_/_0.08)]">
			{/* Header */}
			<div className="border-border flex h-8 items-center gap-1.5 border-b px-2">
				<span className="flex-1 truncate text-sm font-medium">{name}</span>
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
			<div className="border-border space-y-1 border-b p-3">
				{status ? (
					<>
						<RepoStatusBadge status={status} className="text-sm" />
						{status.repo_state_label && (
							<div className="text-muted-foreground text-xs">
								{status.repo_state_label}
							</div>
						)}
						<div className="text-muted-foreground text-xs">
							Last sync: {formatLastSync(status.last_sync_time)}
						</div>
						{status.error && isConflictCategory ? (
							<ConflictPanel idx={idx} config={config} status={status} />
						) : status.error ? (
							<div className="mt-1 space-y-1 rounded bg-red-50 p-1.5 text-xs text-red-700 dark:bg-red-950/20 dark:text-red-400">
								<div className="font-medium">
									{ERROR_LABELS[status.error.category] ?? "Sync error"}
								</div>
								<div className="text-red-600 dark:text-red-500">
									{status.error.message}
								</div>
								{ERROR_HINTS[status.error.category] && (
									<div className="text-red-500/80 italic dark:text-red-400/70">
										{ERROR_HINTS[status.error.category]}
									</div>
								)}
							</div>
						) : null}
					</>
				) : (
					<div className="text-muted-foreground text-sm">Loading…</div>
				)}
			</div>

			{/* Details */}
			<div className="border-border text-muted-foreground space-y-1 border-b p-3 text-xs">
				<div className="flex gap-2">
					<span className="w-14 shrink-0">Path</span>
					<span className="truncate font-mono">{config.repo_path}</span>
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
					<div className="text-muted-foreground py-4 text-center">
						No log entries yet
					</div>
				) : (
					logs.map((entry, i) => (
						<div
							key={i}
							className="mb-0.5 flex gap-1.5 leading-relaxed whitespace-nowrap"
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
