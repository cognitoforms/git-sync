import { ConflictFileContentPayload } from "@/bindings";
import { ContentConflictView } from "./ContentConflictView";
import { DeleteConflictView } from "./DeleteConflictView";
import { Separator } from "@/components/ui/separator";

export type MergeConflictViewProps = {
	file: ConflictFileContentPayload;
	resolvedContent?: string;
	isDeleted?: boolean;
	onResolvedChange: (resolved: { content: string; deleted: boolean }) => void;
	onConflictsResolvedChange: (resolved: boolean) => void;
};

export function MergeConflictView(props: MergeConflictViewProps) {
	const { file } = props;
	const kind = file.conflict_kind.type;

	function handleCtrChange(content: string) {
		props.onResolvedChange({ content, deleted: false });
	}

	function handleKeepFile(content: string) {
		props.onResolvedChange({ content, deleted: false });
		props.onConflictsResolvedChange(true);
	}

	function handleDeleteFile() {
		props.onResolvedChange({ content: "", deleted: true });
		props.onConflictsResolvedChange(true);
	}

	return (
		<>
			{/* File Path Header */}
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

			<Separator />

			{kind === "deleted_by_us" || kind === "deleted_by_them" ? (
				<DeleteConflictView
					file={file}
					isDeleted={props.isDeleted}
					onKeepFile={handleKeepFile}
					onDeleteFile={handleDeleteFile}
				/>
			) : (
				<ContentConflictView
					file={file}
					resolvedContent={props.resolvedContent}
					onCtrChange={handleCtrChange}
					onConflictsResolvedChange={props.onConflictsResolvedChange}
				/>
			)}
		</>
	);
}
