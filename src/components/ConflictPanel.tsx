import { FolderOpen, Code, Warning, GitBranch } from "@phosphor-icons/react";
import {
	useConflictInfo,
	useResolveConflict,
	useRevealInFinder,
	useOpenVSCode,
} from "@/hooks/queries";
import type { RepoStatus } from "@/bindings";
import type { ResolvedRepo } from "@/hooks/queries";

interface Props {
	idx: number;
	config: ResolvedRepo;
	status: RepoStatus;
}

export default function ConflictPanel({ idx, config, status }: Props) {
	const category = status.error?.category;
	const isConflictBranch = category === "conflict_branch";
	const isConflict = category === "conflict";

	const { data: conflictInfo } = useConflictInfo(
		idx,
		isConflict || isConflictBranch,
	);
	const resolveConflict = useResolveConflict();
	const revealInFinder = useRevealInFinder();
	const openVSCode = useOpenVSCode();

	if (!isConflict && !isConflictBranch) return null;

	const files = conflictInfo?.conflicted_files ?? [];
	const branchName =
		conflictInfo?.conflict_branch_name ??
		(status.error?.category === "conflict_branch" ? status.error.branch : null);
	const targetBranch = conflictInfo?.target_branch ?? "main";

	if (isConflictBranch) {
		return (
			<div className="mt-1 space-y-2 rounded bg-amber-50 p-2 text-xs text-amber-800 dark:bg-amber-950/20 dark:text-amber-300">
				<div className="flex items-center gap-1.5 font-medium">
					<GitBranch weight="bold" className="shrink-0" />
					Your changes were saved to a backup branch
				</div>
				<p className="text-amber-700 dark:text-amber-400">
					Someone else changed the same files on{" "}
					<span className="font-mono font-medium">{targetBranch}</span> at the
					same time. Git Sync saved your work to{" "}
					{branchName ? (
						<span className="font-mono font-medium">{branchName}</span>
					) : (
						"a backup branch"
					)}{" "}
					and will merge it back automatically when possible.
				</p>
				{files.length > 0 && (
					<ul className="space-y-0.5">
						{files.map((f) => (
							<li
								key={f}
								className="font-mono text-amber-700 dark:text-amber-400"
							>
								{f}
							</li>
						))}
					</ul>
				)}
				<div className="flex flex-wrap gap-1.5 pt-0.5">
					<ActionButton
						onClick={() =>
							resolveConflict.mutate({
								index: idx,
								strategy: "abandon_conflict_branch",
							})
						}
						pending={resolveConflict.isPending}
						variant="amber"
					>
						Discard my changes, return to {targetBranch}
					</ActionButton>
				</div>
				<div className="flex flex-wrap gap-1.5">
					<ExternalButton
						onClick={() => revealInFinder.mutate(config.repo_path)}
						icon={<FolderOpen weight="bold" />}
					>
						Reveal in Finder
					</ExternalButton>
					<ExternalButton
						onClick={() => openVSCode.mutate(config.repo_path)}
						icon={<Code weight="bold" />}
					>
						Open in VS Code
					</ExternalButton>
				</div>
			</div>
		);
	}

	// Direct conflict: rebase was attempted and aborted
	return (
		<div className="mt-1 space-y-2 rounded bg-red-50 p-2 text-xs text-red-800 dark:bg-red-950/20 dark:text-red-300">
			<div className="flex items-center gap-1.5 font-medium">
				<Warning weight="bold" className="shrink-0" />
				Merge conflict — your files need attention
			</div>
			<p className="text-red-700 dark:text-red-400">
				Two sets of changes conflict on the same lines and can't be merged
				automatically. You'll need to choose which version to keep for each
				affected file.
			</p>
			{files.length > 0 && (
				<ul className="space-y-0.5">
					{files.map((f) => (
						<li key={f} className="font-mono text-red-600 dark:text-red-400">
							{f}
						</li>
					))}
				</ul>
			)}
			<div className="flex flex-wrap gap-1.5 pt-0.5">
				<ActionButton
					onClick={() =>
						resolveConflict.mutate({ index: idx, strategy: "keep_mine" })
					}
					pending={resolveConflict.isPending}
					variant="red"
				>
					Keep My Version
				</ActionButton>
				<ActionButton
					onClick={() =>
						resolveConflict.mutate({ index: idx, strategy: "accept_remote" })
					}
					pending={resolveConflict.isPending}
					variant="red"
				>
					Accept Remote Version
				</ActionButton>
			</div>
			<div className="flex flex-wrap gap-1.5">
				<ExternalButton
					onClick={() => revealInFinder.mutate(config.repo_path)}
					icon={<FolderOpen weight="bold" />}
				>
					Reveal in Finder
				</ExternalButton>
				<ExternalButton
					onClick={() => openVSCode.mutate(config.repo_path)}
					icon={<Code weight="bold" />}
				>
					Open in VS Code
				</ExternalButton>
			</div>
		</div>
	);
}

function ActionButton({
	onClick,
	pending,
	variant,
	children,
}: {
	onClick: () => void;
	pending: boolean;
	variant: "red" | "amber";
	children: React.ReactNode;
}) {
	const cls =
		variant === "amber"
			? "bg-amber-200 text-amber-900 hover:bg-amber-300 dark:bg-amber-800/40 dark:text-amber-200 dark:hover:bg-amber-800/60"
			: "bg-red-200 text-red-900 hover:bg-red-300 dark:bg-red-800/40 dark:text-red-200 dark:hover:bg-red-800/60";
	return (
		<button
			className={`rounded px-2 py-0.5 font-medium disabled:opacity-50 ${cls}`}
			onClick={onClick}
			disabled={pending}
		>
			{children}
		</button>
	);
}

function ExternalButton({
	onClick,
	icon,
	children,
}: {
	onClick: () => void;
	icon: React.ReactNode;
	children: React.ReactNode;
}) {
	return (
		<button
			className="flex items-center gap-1 rounded bg-black/5 px-2 py-0.5 hover:bg-black/10 dark:bg-white/10 dark:hover:bg-white/15"
			onClick={onClick}
		>
			{icon}
			{children}
		</button>
	);
}
