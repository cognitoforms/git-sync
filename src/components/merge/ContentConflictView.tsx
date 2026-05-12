import { ConflictFileContentPayload } from "@/bindings";
import MisMergeEditor from "./MisMergeEditor";

type ContentFile = Extract<ConflictFileContentPayload, { type: "content" }>;

interface ContentConflictViewProps {
	file: ContentFile;
	resolvedContent: string | undefined;
	onCtrChange: (content: string) => void;
	onConflictsResolvedChange: (resolved: boolean) => void;
}

export function ContentConflictView({
	file,
	resolvedContent,
	onCtrChange,
	onConflictsResolvedChange,
}: ContentConflictViewProps) {
	return (
		<>
			{/* Column labels */}
			<div className="text-muted-foreground flex shrink-0 border-b text-xs">
				<div className="flex-1 px-3 py-1 font-medium">Upstream changes</div>
				<div className="w-10 shrink-0" />
				<div className="flex-1 px-3 py-1 font-medium">Resolved (editable)</div>
				<div className="w-10 shrink-0" />
				<div className="flex-1 px-3 py-1 font-medium">My changes</div>
			</div>

			<div className="flex-1 overflow-auto">
				<MisMergeEditor
					key={file.path}
					lhs={file.theirs}
					ctr={resolvedContent ?? file.base ?? ""}
					rhs={file.ours}
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
