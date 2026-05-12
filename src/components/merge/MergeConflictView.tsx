import { ConflictFileContentPayload } from "@/bindings";
import { ContentConflictView } from "./ContentConflictView";
import { DeleteConflictView } from "./DeleteConflictView";
import { RenameConflictView } from "./RenameConflictView";
import { Separator } from "@/components/ui/separator";
import type { Resolution } from "./MergeEditorModal";

export type MergeConflictViewProps = {
	file: ConflictFileContentPayload;
	resolvedContent?: string;
	isDeleted?: boolean;
	chosenPath?: string;
	onResolvedChange: (resolved: Resolution) => void;
	onConflictsResolvedChange: (resolved: boolean) => void;
	onChoosePath: (chosen: string, discarded: string) => void;
};

export function MergeConflictView(props: MergeConflictViewProps) {
	const { file } = props;
	const kind = file.type;

	function handleCtrChange(content: string) {
		props.onResolvedChange({ kind: "written", content });
	}

	function handleKeepFile(content: string) {
		props.onResolvedChange({ kind: "written", content });
		props.onConflictsResolvedChange(true);
	}

	function handleDeleteFile() {
		props.onResolvedChange({ kind: "deleted" });
		props.onConflictsResolvedChange(true);
	}

	// Build the path header depending on the conflict variant.
	function renderPathHeader() {
		if (kind === "rename_rename") {
			return (
				<>
					<span className="opacity-60">{file.our_path}</span>
					<span className="px-0.5 opacity-40">↔</span>
					<span>{file.their_path}</span>
				</>
			);
		}
		if (kind === "content" && file.their_path != null) {
			return (
				<>
					<span className="opacity-60">{file.their_path}</span>
					<span className="opacity-40">→</span>
					<span>{file.path}</span>
				</>
			);
		}
		return <>{file.path}</>;
	}

	return (
		<>
			{/* File Path Header */}
			<div className="text-muted-foreground flex h-8 items-center gap-1.5 px-3 font-mono text-xs">
				{renderPathHeader()}
			</div>

			<Separator />

			{kind === "deleted_by_us" || kind === "deleted_by_them" ? (
				<DeleteConflictView
					file={file}
					isDeleted={props.isDeleted}
					onKeepFile={handleKeepFile}
					onDeleteFile={handleDeleteFile}
				/>
			) : kind === "rename_rename" ? (
				<RenameConflictView
					file={file}
					chosenPath={props.chosenPath}
					resolvedContent={props.resolvedContent}
					onChoosePath={props.onChoosePath}
					onCtrChange={handleCtrChange}
					onConflictsResolvedChange={props.onConflictsResolvedChange}
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
