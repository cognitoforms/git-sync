import { useEffect, useMemo, useRef, useState } from "react";
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
import { TreeView, type TreeDataItem } from "@/components/tree-view";
import { cn } from "@/lib/utils";

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

// ── Component ─────────────────────────────────────────────────────────────────

export default function MergeEditorModal({ isOpen, onClose, repoIdx }: Props) {
	const { data: files, isLoading } = useConflictFilesContent(repoIdx, isOpen);
	const completeMerge = useCompleteMerge();

	const [selectedPath, setSelectedPath] = useState<string | null>(null);
	const [resolvedMap, setResolvedMap] = useState<Record<string, string>>({});
	const [sidebarWidth, setSidebarWidth] = useState(224);

	const isDragging = useRef(false);
	const dragStartX = useRef(0);
	const dragStartWidth = useRef(0);

	useEffect(() => {
		function handleMouseMove(e: MouseEvent) {
			if (!isDragging.current) return;
			const delta = e.clientX - dragStartX.current;
			setSidebarWidth(
				Math.max(120, Math.min(480, dragStartWidth.current + delta)),
			);
		}
		function handleMouseUp() {
			isDragging.current = false;
		}
		document.addEventListener("mousemove", handleMouseMove);
		document.addEventListener("mouseup", handleMouseUp);
		return () => {
			document.removeEventListener("mousemove", handleMouseMove);
			document.removeEventListener("mouseup", handleMouseUp);
		};
	}, []);

	useEffect(() => {
		if (!files || files.length === 0) return;
		setSelectedPath(files[0].path);
	}, [files]);

	const filePaths = files?.map((f) => f.path) ?? [];
	const treeData = buildTreeItems(filePaths);
	const currentFile = files?.find((f) => f.path === selectedPath);

	const isFileResolved = (path: string) => !!resolvedMap[path];
	const resolvedCount = useMemo(
		() => filePaths.filter((p) => !!resolvedMap[p]).length,
		[filePaths, resolvedMap],
	);
	const allResolved =
		files != null && files.length > 0 && resolvedCount === files.length;

	function handleSelectChange(item: TreeDataItem | undefined) {
		if (!item?.id || item.children != null) return;
		setSelectedPath(item.id);
	}

	function handleCtrChange(content: string) {
		if (!currentFile) return;
		setResolvedMap((m) => ({ ...m, [currentFile.path]: content }));
	}

	async function handleCompleteMerge() {
		if (!files) return;
		const resolved = files.map((f) => ({
			path: f.path,
			content: resolvedMap[f.path] ?? f.base,
		}));
		await completeMerge.mutateAsync({ index: repoIdx, resolved });
		onClose();
	}

	function handleDividerMouseDown(e: React.MouseEvent) {
		isDragging.current = true;
		dragStartX.current = e.clientX;
		dragStartWidth.current = sidebarWidth;
		e.preventDefault();
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
				<div className="flex min-h-0 flex-1">
					{/* File tree sidebar */}
					<div
						className="flex shrink-0 flex-col"
						style={{ width: sidebarWidth }}
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
					</div>

					{/* Drag handle */}
					<div
						className="bg-border hover:bg-primary/40 w-px shrink-0 cursor-col-resize transition-colors"
						onMouseDown={handleDividerMouseDown}
					/>

					{/* Editor */}
					<div className="flex min-w-0 flex-1 flex-col">
						{currentFile ? (
							<>
								<div className="text-muted-foreground flex h-8 items-center px-3 font-mono text-xs">
									{currentFile.path}
								</div>

								<Separator />

								{/* Column labels */}
								<div className="text-muted-foreground flex shrink-0 border-b text-xs">
									<div className="flex-1 px-3 py-1 font-medium">
										Upstream changes
									</div>
									<div className="w-10 shrink-0" />
									<div className="flex-1 px-3 py-1 font-medium">
										Resolved (editable)
									</div>
									<div className="w-10 shrink-0" />
									<div className="flex-1 px-3 py-1 font-medium">My changes</div>
								</div>
								<div className="flex-1 overflow-auto">
									<MergeEditor
										lhs={currentFile.theirs}
										ctr={resolvedMap[currentFile.path] ?? currentFile.base}
										rhs={currentFile.ours}
										onCtrChange={handleCtrChange}
										lhsEditable={false}
										rhsEditable={false}
										ctrEditable={true}
										wrapLines={true}
										className="h-full!"
									/>
								</div>
							</>
						) : (
							!isLoading && (
								<div className="text-muted-foreground flex flex-1 items-center justify-center text-sm">
									No conflicting files found.
								</div>
							)
						)}
					</div>
				</div>

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
