import { useMemo, useState } from "react";
import { useConflictFilesContent, useCompleteMerge } from "@/hooks/queries";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import {
	Dialog,
	DialogContent,
	DialogTitle,
	DialogHeader,
	DialogFooter,
} from "@/components/ui/dialog";
import {
	ResizablePanelGroup,
	ResizablePanel,
	ResizableHandle,
} from "@/components/ui/resizable";
import type {
	ConflictFileContentPayload,
	ResolvedFilePayload,
} from "@/bindings";
import { MergeEditorFileTree } from "./MergeEditorFileTree";
import { MergeConflictView } from "./MergeConflictView";

interface Props {
	isOpen: boolean;
	onClose: () => void;
	repoIdx: number;
}

// ── Resolution state ──────────────────────────────────────────────────────────

export type Resolution =
	| { kind: "written"; content: string }
	| { kind: "deleted" }
	| {
			kind: "rename_resolved";
			chosenPath: string;
			discardedPath: string;
			content: string;
	  };

// Returns the stable key used to identify a conflict file across maps.
// rename_rename has no single `path` field, so we use our_path.
function fileKey(f: ConflictFileContentPayload): string {
	return f.type === "rename_rename" ? f.our_path : f.path;
}

export default function MergeEditorModal({ isOpen, onClose, repoIdx }: Props) {
	const { data: files, isLoading } = useConflictFilesContent(repoIdx, isOpen);
	const completeMerge = useCompleteMerge();

	const [selectedPath, setSelectedPath] = useState<string | null>(null);
	const [resolvedMap, setResolvedMap] = useState<Record<string, Resolution>>(
		{},
	);
	const [fileResolvedMap, setFileResolvedMap] = useState<
		Record<string, boolean>
	>({});

	const filePaths = files?.map(fileKey) ?? [];
	const currentFile =
		files?.find((f) => fileKey(f) === selectedPath) ?? files?.[0] ?? null;

	const resolvedCount = useMemo(
		() => filePaths.filter((p) => fileResolvedMap[p] === true).length,
		[filePaths, fileResolvedMap],
	);
	const allResolved =
		files != null && files.length > 0 && resolvedCount === files.length;

	const currentRes =
		currentFile != null ? resolvedMap[fileKey(currentFile)] : undefined;

	async function handleCompleteMerge() {
		if (!files) return;
		const resolved: ResolvedFilePayload[] = files.map((f) => {
			const key = fileKey(f);
			const res = resolvedMap[key];
			if (!res) {
				const fallbackContent =
					f.type === "content" || f.type === "deleted_by_them"
						? (f.base ?? "")
						: "";
				return { type: "written", path: key, content: fallbackContent };
			}
			if (res.kind === "written") {
				return { type: "written", path: key, content: res.content };
			}
			if (res.kind === "deleted") {
				return { type: "deleted", path: key };
			}
			return {
				type: "rename_resolved",
				chosen_path: res.chosenPath,
				discarded_path: res.discardedPath,
				content: res.content,
			};
		});
		await completeMerge.mutateAsync({ index: repoIdx, resolved });
		onClose();
	}

	function handleChoosePath(
		f: Extract<ConflictFileContentPayload, { type: "rename_rename" }>,
		chosen: string,
		discarded: string,
	) {
		const content = chosen === f.our_path ? f.ours : f.theirs;
		setResolvedMap((m) => ({
			...m,
			[f.our_path]: {
				kind: "rename_resolved",
				chosenPath: chosen,
				discardedPath: discarded,
				content,
			},
		}));
		// For pure renames (identical content), mark resolved immediately.
		if (f.ours === f.theirs) {
			setFileResolvedMap((m) => ({ ...m, [f.our_path]: true }));
		}
	}

	return (
		<Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
			<DialogContent
				showCloseButton={false}
				className="top-11 right-4 bottom-4 left-4 flex w-auto max-w-none translate-0 flex-col gap-0 p-0 sm:max-w-none"
			>
				{/* Header */}
				<DialogHeader className="flex-row items-center border-b px-3 py-2">
					<DialogTitle className="flex-1 text-sm">
						Resolve Conflicts
					</DialogTitle>
					<Button
						variant="ghost"
						size="icon-sm"
						onClick={onClose}
						aria-label="Close"
					>
						<span aria-hidden>✕</span>
					</Button>
				</DialogHeader>

				{/* Body */}
				<ResizablePanelGroup
					orientation="horizontal"
					className="min-h-0 flex-1"
				>
					{/* File tree sidebar */}
					<ResizablePanel
						defaultSize={224}
						minSize={120}
						maxSize={480}
						className="flex flex-col"
					>
						<div className="text-muted-foreground flex h-8 items-center px-3 text-xs font-medium">
							Files ({filePaths.length})
						</div>
						<Separator />
						<div className="flex-1 overflow-auto">
							{isLoading && (
								<div className="text-muted-foreground px-3 py-2 text-xs">
									Loading…
								</div>
							)}

							<MergeEditorFileTree
								filePaths={filePaths}
								resolvedMap={fileResolvedMap}
								selectedPath={selectedPath}
								setSelectedPath={setSelectedPath}
							/>
						</div>
					</ResizablePanel>

					<ResizableHandle />

					{/* Editor */}
					<ResizablePanel className="flex flex-col">
						{currentFile == null && !isLoading && (
							<div className="text-muted-foreground flex flex-1 items-center justify-center text-sm">
								No conflicting files found.
							</div>
						)}
						{currentFile != null && (
							<MergeConflictView
								file={currentFile}
								resolvedContent={
									currentRes?.kind === "written" ||
									currentRes?.kind === "rename_resolved"
										? currentRes.content
										: undefined
								}
								isDeleted={currentRes?.kind === "deleted"}
								chosenPath={
									currentRes?.kind === "rename_resolved"
										? currentRes.chosenPath
										: undefined
								}
								onResolvedChange={(res) =>
									setResolvedMap((m) => ({
										...m,
										[fileKey(currentFile)]: res,
									}))
								}
								onConflictsResolvedChange={(resolved) =>
									setFileResolvedMap((m) => ({
										...m,
										[fileKey(currentFile)]: resolved,
									}))
								}
								onChoosePath={(chosen, discarded) => {
									if (currentFile.type === "rename_rename") {
										handleChoosePath(currentFile, chosen, discarded);
									}
								}}
							/>
						)}
					</ResizablePanel>
				</ResizablePanelGroup>

				{/* Footer */}
				<DialogFooter className="flex-row items-center border-t px-4 py-2.5">
					<span className="text-muted-foreground flex-1 text-xs">
						{resolvedCount} of {filePaths.length} files resolved
					</span>
					<Button variant="outline" size="sm" onClick={onClose}>
						Cancel
					</Button>
					<Button
						size="sm"
						onClick={handleCompleteMerge}
						disabled={!allResolved || completeMerge.isPending || !files?.length}
					>
						{completeMerge.isPending ? "Merging…" : "Complete Merge"}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
