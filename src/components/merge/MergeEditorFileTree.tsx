import { TreeView, type TreeDataItem } from "@/components/ui/tree-view";
import { cn } from "@/lib/utils";

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

export type MergeEditorFileTreeProps = {
	filePaths: string[];
	resolvedMap: Record<string, boolean>;
	selectedPath?: string | null;
	setSelectedPath: (path: string) => void;
};

export function MergeEditorFileTree(props: MergeEditorFileTreeProps) {
	const treeData = buildTreeItems(props.filePaths);

	const isFileResolved = (path: string) => props.resolvedMap[path] === true;
	function handleSelectChange(item: TreeDataItem | undefined) {
		if (!item?.id || item.children != null) return;
		props.setSelectedPath(item.id);
	}

	return (
		<>
			{treeData.length > 0 && (
				<TreeView
					data={treeData}
					initialSelectedItemId={props.selectedPath ?? props.filePaths[0]}
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
										isFileResolved(item.id) ? "bg-green-500" : "bg-amber-400",
									)}
								/>
							)}
							<span className="truncate">{item.name}</span>
						</span>
					)}
				/>
			)}
		</>
	);
}
