import { cn } from "@/lib/utils";
import type { RepoStatus } from "@/types";
import StatusDot from "./StatusDot";

export const ERROR_LABELS: Record<string, string> = {
	auth: "Auth error",
	network: "Network error",
	conflict: "Merge conflict",
	conflict_branch: "Conflict branch",
	config: "Config error",
	state: "Bad repo state",
	unknown: "Sync error",
};

interface Props {
	status: RepoStatus;
	className?: string;
}

const WARNING_CATEGORIES = new Set(["conflict", "conflict_branch"]);

function errorLevel(category: string): "warning" | "critical" {
	return WARNING_CATEGORIES.has(category) ? "warning" : "critical";
}

export default function RepoStatusBadge({ status, className }: Props) {
	const level = status.error ? errorLevel(status.error.category) : undefined;
	const label = status.is_syncing
		? "Syncing…"
		: status.error
			? (ERROR_LABELS[status.error.category] ?? "Sync error")
			: status.sync_state_label;

	return (
		<div className={cn("flex items-center gap-1.5", className)}>
			<StatusDot
				id={status.sync_state_id}
				syncing={status.is_syncing}
				errorLevel={level}
			/>
			<span
				className={
					!level || status.is_syncing
						? "text-foreground"
						: level === "warning"
							? "text-amber-600 dark:text-amber-400"
							: "text-red-600 dark:text-red-400"
				}
			>
				{label}
			</span>
		</div>
	);
}
