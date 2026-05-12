import { ConflictFileContentPayload } from "@/bindings";
import { Button } from "@/components/ui/button";
import MisMergeEditor from "./MisMergeEditor";

type RenameFile = Extract<
	ConflictFileContentPayload,
	{ type: "rename_rename" }
>;

interface RenameConflictViewProps {
	file: RenameFile;
	chosenPath: string | undefined;
	resolvedContent: string | undefined;
	onChoosePath: (chosen: string, discarded: string) => void;
	onCtrChange: (content: string) => void;
	onConflictsResolvedChange: (resolved: boolean) => void;
}

export function RenameConflictView({
	file,
	chosenPath,
	resolvedContent,
	onChoosePath,
	onCtrChange,
	onConflictsResolvedChange,
}: RenameConflictViewProps) {
	const contentDiffers = file.ours !== file.theirs;

	function handleChoose(chosen: string) {
		const discarded =
			chosen === file.our_path ? file.their_path : file.our_path;
		onChoosePath(chosen, discarded);
	}

	return (
		<div className="flex flex-1 flex-col overflow-auto">
			{/* Explanation */}
			<div className="shrink-0 p-4">
				<div className="rounded-md border border-amber-400/40 bg-amber-400/10 p-3 text-sm text-amber-700 dark:text-amber-300">
					Both sides renamed this file to different paths. Choose which name to
					keep.
				</div>
			</div>

			{/* Path picker */}
			<div className="flex shrink-0 flex-col gap-2 px-4 pb-4">
				<div className="text-muted-foreground text-xs font-medium">
					Choose final path
				</div>
				<div className="flex gap-2">
					<Button
						size="sm"
						variant={chosenPath === file.our_path ? "default" : "outline"}
						onClick={() => handleChoose(file.our_path)}
						className="flex-1 justify-start font-mono text-xs"
					>
						{file.our_path}
						<span className="text-muted-foreground ml-1.5 text-xs font-normal">
							(local)
						</span>
					</Button>
					<Button
						size="sm"
						variant={chosenPath === file.their_path ? "default" : "outline"}
						onClick={() => handleChoose(file.their_path)}
						className="flex-1 justify-start font-mono text-xs"
					>
						{file.their_path}
						<span className="text-muted-foreground ml-1.5 text-xs font-normal">
							(remote)
						</span>
					</Button>
				</div>
			</div>

			{/* Content editor — only shown when content also differs */}
			{contentDiffers && (
				<>
					<div className="text-muted-foreground flex shrink-0 border-t border-b text-xs">
						<div className="flex-1 px-3 py-1 font-medium">Upstream changes</div>
						<div className="w-10 shrink-0" />
						<div className="flex-1 px-3 py-1 font-medium">
							Resolved (editable)
						</div>
						<div className="w-10 shrink-0" />
						<div className="flex-1 px-3 py-1 font-medium">My changes</div>
					</div>

					<div className="flex-1 overflow-auto">
						<MisMergeEditor
							key={file.our_path}
							lhs={file.theirs}
							ctr={resolvedContent ?? file.base ?? ""}
							rhs={file.ours}
							onCtrChange={onCtrChange}
							onConflictsResolvedChange={(resolved) => {
								// Only propagate resolved state if a path has also been chosen.
								if (chosenPath != null) {
									onConflictsResolvedChange(resolved);
								}
							}}
							lhsEditable={false}
							rhsEditable={false}
							ctrEditable={true}
							wrapLines={true}
							className="h-full!"
						/>
					</div>
				</>
			)}
		</div>
	);
}
