import { useState } from "react";
import { platform } from "@tauri-apps/plugin-os";
import { FolderOpen, Code, Warning, GitBranch } from "@phosphor-icons/react";
import {
	useConflictInfo,
	useResolveConflict,
	useRevealInFinder,
	useOpenVSCode,
} from "@/hooks/queries";
import type { RepoStatus } from "@/bindings";
import type { ResolvedRepo } from "@/hooks/queries";
import { Button } from "@/components/ui/button";
import MergeEditorModal from "./MergeEditorModal";

interface Props {
	idx: number;
	config: ResolvedRepo;
	status: RepoStatus;
}

export default function ConflictPanel({ idx, config, status }: Props) {
	const [mergeEditorOpen, setMergeEditorOpen] = useState(false);
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
	const p = platform();
	const revealLabel =
		p === "macos"
			? "Reveal in Finder"
			: p === "windows"
				? "Show in Explorer"
				: "Open in File Manager";

	if (!isConflict && !isConflictBranch) return null;

	const files = conflictInfo?.conflicted_files ?? [];
	const branchName =
		conflictInfo?.conflict_branch_name ??
		(status.error?.category === "conflict_branch" ? status.error.branch : null);
	const targetBranch = conflictInfo?.target_branch ?? "main";

	const color = isConflictBranch ? "amber" : "red";
	const bannerCls =
		color === "amber"
			? "bg-amber-50 text-amber-800 dark:bg-amber-950/20 dark:text-amber-300"
			: "bg-red-50 text-red-800 dark:bg-red-950/20 dark:text-red-300";
	const textCls =
		color === "amber"
			? "text-amber-700 dark:text-amber-400"
			: "text-red-700 dark:text-red-400";
	const fileCls =
		color === "amber"
			? "font-mono text-amber-700 dark:text-amber-400"
			: "font-mono text-red-600 dark:text-red-400";
	const btnCls =
		color === "amber"
			? "bg-amber-200 text-amber-900 hover:bg-amber-300 dark:bg-amber-800/40 dark:text-amber-200 dark:hover:bg-amber-800/60"
			: "bg-red-200 text-red-900 hover:bg-red-300 dark:bg-red-800/40 dark:text-red-200 dark:hover:bg-red-800/60";

	return (
		<>
			<div className={`mt-1 space-y-2 rounded p-2 text-xs ${bannerCls}`}>
				<div className="flex items-center gap-1.5 font-medium">
					{isConflictBranch ? (
						<GitBranch weight="bold" className="shrink-0" />
					) : (
						<Warning weight="bold" className="shrink-0" />
					)}
					{isConflictBranch
						? "Your changes were saved to a backup branch"
						: "Merge conflict — your files need attention"}
				</div>
				<p className={textCls}>
					{isConflictBranch ? (
						<>
							Someone else changed the same files on{" "}
							<span className="font-mono font-medium">{targetBranch}</span> at
							the same time. Git Sync saved your work to{" "}
							{branchName ? (
								<span className="font-mono font-medium">{branchName}</span>
							) : (
								"a backup branch"
							)}{" "}
							and will merge it back automatically when possible.
						</>
					) : (
						"Two sets of changes conflict on the same lines and can't be merged automatically. You'll need to choose which version to keep for each affected file."
					)}
				</p>
				{files.length > 0 && (
					<ul className="space-y-0.5">
						{files.map((f) => (
							<li key={f} className={fileCls}>
								{f}
							</li>
						))}
					</ul>
				)}
				<div className="flex flex-wrap gap-1.5 pt-0.5">
					<Button
						size="sm"
						className={`h-auto cursor-pointer rounded px-2 py-0.5 font-medium ${btnCls}`}
						onClick={() => setMergeEditorOpen(true)}
					>
						Resolve conflicts
					</Button>
					{isConflictBranch ? (
						<Button
							size="sm"
							className={`h-auto cursor-pointer rounded px-2 py-0.5 font-medium ${btnCls}`}
							onClick={() =>
								resolveConflict.mutate({
									index: idx,
									strategy: "abandon_conflict_branch",
								})
							}
							disabled={resolveConflict.isPending}
						>
							Discard changes, return to {targetBranch}
						</Button>
					) : (
						<>
							<Button
								size="sm"
								className={`h-auto cursor-pointer rounded px-2 py-0.5 font-medium ${btnCls}`}
								onClick={() =>
									resolveConflict.mutate({
										index: idx,
										strategy: "keep_mine",
									})
								}
								disabled={resolveConflict.isPending}
							>
								Keep My Version
							</Button>
							<Button
								size="sm"
								className={`h-auto cursor-pointer rounded px-2 py-0.5 font-medium ${btnCls}`}
								onClick={() =>
									resolveConflict.mutate({
										index: idx,
										strategy: "accept_remote",
									})
								}
								disabled={resolveConflict.isPending}
							>
								Accept Remote Version
							</Button>
						</>
					)}
				</div>
				<div className="flex flex-wrap gap-1.5">
					<Button
						variant="ghost"
						size="sm"
						className="h-auto gap-1 rounded bg-black/5 px-2 py-0.5 hover:bg-black/10 dark:bg-white/10 dark:hover:bg-white/15"
						onClick={() => revealInFinder.mutate(config.repo_path)}
					>
						<FolderOpen weight="bold" />
						{revealLabel}
					</Button>
					<Button
						variant="ghost"
						size="sm"
						className="h-auto gap-1 rounded bg-black/5 px-2 py-0.5 hover:bg-black/10 dark:bg-white/10 dark:hover:bg-white/15"
						onClick={() => openVSCode.mutate(config.repo_path)}
					>
						<Code weight="bold" />
						Open in VS Code
					</Button>
				</div>
			</div>
			<MergeEditorModal
				isOpen={mergeEditorOpen}
				onClose={() => setMergeEditorOpen(false)}
				repoIdx={idx}
			/>
		</>
	);
}
