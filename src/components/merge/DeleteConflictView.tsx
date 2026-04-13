import { ConflictFileContentPayload } from "@/bindings";
import { Button } from "@/components/ui/button";

interface DeleteConflictViewProps {
	file: ConflictFileContentPayload;
	isDeleted?: boolean;
	onKeepFile: (content: string) => void;
	onDeleteFile: () => void;
}

export function DeleteConflictView({
	file,
	isDeleted,
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
					variant={isDeleted === false ? "default" : "outline"}
					onClick={() => onKeepFile(survivingContent ?? "")}
				>
					{keepLabel}
				</Button>
				<Button
					size="sm"
					variant={isDeleted === true ? "default" : "outline"}
					onClick={() => onDeleteFile()}
				>
					Delete File
				</Button>
			</div>
		</div>
	);
}
