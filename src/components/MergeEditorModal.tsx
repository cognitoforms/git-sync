import { useMemo, useState } from "react";
import MergeEditor from "./MergeEditor";
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
import { TreeView, type TreeDataItem } from "@/components/ui/tree-view";
import { cn } from "@/lib/utils";
import type { ConflictFileContentPayload } from "@/bindings";

interface Props {
	isOpen: boolean;
	onClose: () => void;
	repoIdx: number;
}

// ── Tree building ─────────────────────────────────────────────────────────────

type DirNode = { dirs: Map<string, DirNode>; files: string[] };

function buildTreeItems(filePaths: string[]): TreeDataItem[] {
	const root: DirNode = { dirs: new Map(), files: [] };

	for (const path of filePaths) {
		const parts = path.split("/");
		let node = root;
		for (let i = 0; i < parts.length - 1; i++) {
			if (!node.dirs.has(parts[i])) {
				node.dirs.set(parts[i], { dirs: new Map(), files: [] });
			}
			node = node.dirs.get(parts[i])!;
		}
		node.files.push(path);
	}

	function nodeToItems(node: DirNode, prefix: string): TreeDataItem[] {
		const items: TreeDataItem[] = [];

		for (const filePath of node.files) {
			items.push({ id: filePath, name: filePath.split("/").pop() ?? filePath });
		}

		for (const [name, child] of node.dirs) {
			// Collapse single-child-directory chains (e.g. "src/components/")
			let displayName = name;
			let current = child;
			while (current.files.length === 0 && current.dirs.size === 1) {
				const [[childName, childNode]] = [...current.dirs.entries()];
				displayName += "/" + childName;
				current = childNode;
			}
			items.push({
				id: prefix + displayName + "/",
				name: displayName + "/",
				children: nodeToItems(current, prefix + displayName + "/"),
			});
		}

		return items;
	}

	return nodeToItems(root, "");
}

// ── Resolution state ──────────────────────────────────────────────────────────

type Resolution = { content: string; deleted: boolean };

// ── Sub-components ────────────────────────────────────────────────────────────

function FilePathHeader({ file }: { file: ConflictFileContentPayload }) {
	return (
		<div className="text-muted-foreground flex h-8 items-center gap-1.5 px-3 font-mono text-xs">
			{file.their_path != null ? (
				<>
					<span className="opacity-60">{file.their_path}</span>
					<span className="opacity-40">→</span>
					<span>{file.path}</span>
				</>
			) : (
				file.path
			)}
		</div>
	);
}

interface DeleteConflictViewProps {
	file: ConflictFileContentPayload;
	resolvedMap: Record<string, Resolution>;
	onKeepFile: (path: string, content: string) => void;
	onDeleteFile: (path: string) => void;
}

function DeleteConflictView({
	file,
	resolvedMap,
	onKeepFile,
	onDeleteFile,
}: DeleteConflictViewProps) {
	const isDeletedByUs = file.conflict_kind.type === "deleted_by_us";
	const survivingContent = isDeletedByUs ? file.theirs : file.ours;
	const description = isDeletedByUs
		? "This file was deleted locally but modified remotely."
		: "This file was modified locally but deleted remotely.";
	const keepLabel = isDeletedByUs ? "Keep Remote File" : "Keep Local File";

	return (
		<>
			<FilePathHeader file={file} />
			<Separator />
			<div className="flex flex-1 flex-col gap-4 overflow-auto p-4">
				<div className="rounded-md border border-amber-400/40 bg-amber-400/10 p-3 text-sm text-amber-700 dark:text-amber-300">
					{description}
				</div>
				{survivingContent != null && (
					<div className="flex flex-1 flex-col gap-1 overflow-auto">
						<div className="text-muted-foreground text-xs font-medium">
							{isDeletedByUs ? "Remote content" : "Local content"}
						</div>
						<pre className="bg-muted flex-1 overflow-auto rounded-md p-3 font-mono text-xs">
							{survivingContent}
						</pre>
					</div>
				)}
				<div className="flex shrink-0 gap-2">
					<Button
						size="sm"
						variant={
							resolvedMap[file.path]?.deleted === false ? "default" : "outline"
						}
						onClick={() => onKeepFile(file.path, survivingContent ?? "")}
					>
						{keepLabel}
					</Button>
					<Button
						size="sm"
						variant={
							resolvedMap[file.path]?.deleted === true ? "default" : "outline"
						}
						onClick={() => onDeleteFile(file.path)}
					>
						Delete File
					</Button>
				</div>
			</div>
		</>
	);
}

interface ContentConflictViewProps {
	file: ConflictFileContentPayload;
	resolvedContent: string | undefined;
	onCtrChange: (content: string) => void;
	onConflictsResolvedChange: (resolved: boolean) => void;
}

function ContentConflictView({
	file,
	resolvedContent,
	onCtrChange,
	onConflictsResolvedChange,
}: ContentConflictViewProps) {
	return (
		<>
			<FilePathHeader file={file} />

			<Separator />

			{/* Column labels */}
			<div className="text-muted-foreground flex shrink-0 border-b text-xs">
				<div className="flex-1 px-3 py-1 font-medium">Upstream changes</div>
				<div className="w-10 shrink-0" />
				<div className="flex-1 px-3 py-1 font-medium">Resolved (editable)</div>
				<div className="w-10 shrink-0" />
				<div className="flex-1 px-3 py-1 font-medium">My changes</div>
			</div>
			<div className="flex-1 overflow-auto">
				<MergeEditor
					key={file.path}
					lhs={file.theirs ?? ""}
					ctr={resolvedContent ?? file.base ?? ""}
					rhs={file.ours ?? ""}
					onCtrChange={onCtrChange}
					onConflictsResolvedChange={onConflictsResolvedChange}
					lhsEditable={false}
					rhsEditable={false}
					ctrEditable={true}
					wrapLines={true}
					className="h-full!"
				/>
			</div>
		</>
	);
}

// ── Main component ────────────────────────────────────────────────────────────

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
	const treeData = buildTreeItems(filePaths);
	const currentFile =
		files?.find((f) => f.path === selectedPath) ?? files?.[0] ?? null;

	const isFileResolved = (path: string) => fileResolvedMap[path] === true;
	const resolvedCount = useMemo(
		() => filePaths.filter((p) => fileResolvedMap[p] === true).length,
		[filePaths, fileResolvedMap],
	);
	const allResolved =
		files != null && files.length > 0 && resolvedCount === files.length;

	function handleSelectChange(item: TreeDataItem | undefined) {
		if (!item?.id || item.children != null) return;
		setSelectedPath(item.id);
	}

	function handleCtrChange(content: string) {
		if (!currentFile) return;
		setResolvedMap((m) => ({
			...m,
			[currentFile.path]: { content, deleted: false },
		}));
	}

	function handleConflictsResolvedChange(resolved: boolean) {
		if (!currentFile) return;
		setFileResolvedMap((m) => ({ ...m, [currentFile.path]: resolved }));
	}

	function handleKeepFile(path: string, content: string) {
		setResolvedMap((m) => ({ ...m, [path]: { content, deleted: false } }));
		setFileResolvedMap((m) => ({ ...m, [path]: true }));
	}

	function handleDeleteFile(path: string) {
		setResolvedMap((m) => ({ ...m, [path]: { content: "", deleted: true } }));
		setFileResolvedMap((m) => ({ ...m, [path]: true }));
	}

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

	function renderEditorArea() {
		if (!currentFile) {
			return (
				!isLoading && (
					<div className="text-muted-foreground flex flex-1 items-center justify-center text-sm">
						No conflicting files found.
					</div>
				)
			);
		}

		const kind = currentFile.conflict_kind.type;
		if (kind === "deleted_by_us" || kind === "deleted_by_them") {
			return (
				<DeleteConflictView
					file={currentFile}
					resolvedMap={resolvedMap}
					onKeepFile={handleKeepFile}
					onDeleteFile={handleDeleteFile}
				/>
			);
		}

		return (
			<ContentConflictView
				file={currentFile}
				resolvedContent={resolvedMap[currentFile.path]?.content}
				onCtrChange={handleCtrChange}
				onConflictsResolvedChange={handleConflictsResolvedChange}
			/>
		);
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
							{treeData.length > 0 && (
								<TreeView
									data={treeData}
									initialSelectedItemId={filePaths[0]}
									expandAll={true}
									onSelectChange={handleSelectChange}
									className="p-1"
									renderItem={({ item, isLeaf, isSelected }) => (
										<span
											className={cn(
												"flex min-w-0 items-center gap-1.5 font-mono text-xs",
												isSelected && "font-semibold",
											)}
										>
											{isLeaf && (
												<span
													className={cn(
														"inline-block size-1.5 shrink-0 rounded-full",
														isFileResolved(item.id)
															? "bg-green-500"
															: "bg-amber-400",
													)}
												/>
											)}
											<span className="truncate">{item.name}</span>
										</span>
									)}
								/>
							)}
						</div>
					</ResizablePanel>

					<ResizableHandle />

					{/* Editor */}
					<ResizablePanel className="flex flex-col">
						{renderEditorArea()}
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
