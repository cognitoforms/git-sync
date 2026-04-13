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
import { MergeEditorFileTree } from "./MergeEditorFileTree";
import { MergeConflictView } from "./MergeConflictView";

interface Props {
	isOpen: boolean;
	onClose: () => void;
	repoIdx: number;
}

// ── Resolution state ──────────────────────────────────────────────────────────

type Resolution = { content: string; deleted: boolean };

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

	const filePaths = files?.map((f) => f.path) ?? [];
	const currentFile =
		files?.find((f) => f.path === selectedPath) ?? files?.[0] ?? null;

	const resolvedCount = useMemo(
		() => filePaths.filter((p) => fileResolvedMap[p] === true).length,
		[filePaths, fileResolvedMap],
	);
	const allResolved =
		files != null && files.length > 0 && resolvedCount === files.length;

	async function handleCompleteMerge() {
		if (!files) return;
		const resolved = files.map((f) => {
			const res = resolvedMap[f.path];
			if (!res) {
				return { path: f.path, content: f.base ?? "", deleted: false };
			}
			return { path: f.path, content: res.content, deleted: res.deleted };
		});
		await completeMerge.mutateAsync({ index: repoIdx, resolved });
		onClose();
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
								resolvedContent={resolvedMap[currentFile.path]?.content}
								onResolvedChange={(res) =>
									setResolvedMap((m) => ({
										...m,
										[currentFile.path]: res,
									}))
								}
								onConflictsResolvedChange={(resolved) =>
									setFileResolvedMap((m) => ({
										...m,
										[currentFile.path]: resolved,
									}))
								}
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
