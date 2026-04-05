import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, events } from "./api";
import type {
  AppStatus,
	DiffCommitSummary,
	DiffCompareMode,
	DiffViewData,
	DiffViewerContext,
} from "./bindings";
import TitleBar from "./components/TitleBar";
import { Button } from "./components/ui/button";

const appWindow = getCurrentWindow();

type ViewerMode = "latest_commit" | "latest_author" | "compare_commits";

function formatCommitOption(commit: DiffCommitSummary): string {
	return `${new Date(commit.timestamp).toLocaleString()} · ${commit.author_name} · ${commit.short_sha}`;
}

function buildMode(
	mode: ViewerMode,
	manualFromSha: string,
	manualToSha: string,
): DiffCompareMode {
	if (mode === "latest_author") {
		return { kind: "latest_author" };
	}
	if (mode === "compare_commits") {
		return { kind: "compare_commits", from_sha: manualFromSha, to_sha: manualToSha };
	}
	return { kind: "latest_commit" };
}

export default function DiffViewerApp() {
	const [context, setContext] = useState<DiffViewerContext | null>(null);
	const [isContextLoading, setIsContextLoading] = useState(true);
	const [isViewLoading, setIsViewLoading] = useState(false);
   const [refreshTick, setRefreshTick] = useState(0);
	const [showSidebar, setShowSidebar] = useState(true);
	const [fontSize, setFontSize] = useState(18);
	const [mode, setMode] = useState<ViewerMode>("latest_commit");
    const [lastLiveMode, setLastLiveMode] = useState<Exclude<ViewerMode, "compare_commits">>(
		"latest_commit",
	);
	const [commits, setCommits] = useState<DiffCommitSummary[]>([]);
	const [manualFromSha, setManualFromSha] = useState("");
	const [manualToSha, setManualToSha] = useState("");
	const [selectedFile, setSelectedFile] = useState<string | null>(null);
	const [view, setView] = useState<DiffViewData | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [liveRefreshNotice, setLiveRefreshNotice] = useState<string | null>(null);
 const lastSyncTimeRef = useRef<string | null>(null);
	const diffContainerRef = useRef<HTMLDivElement | null>(null);
	const firstHunkRef = useRef<HTMLDivElement | null>(null);

	const storageKey = useMemo(
		() =>
			context
				? `git-sync-diff-viewer:${context.repo_path}`
				: `git-sync-diff-viewer:${appWindow.label}`,
		[context],
	);

	useEffect(() => {
		let mounted = true;

		commands
			.getDiffViewerContext(appWindow.label)
			.then((ctx: DiffViewerContext | null) => {
				if (!mounted) return;
				setContext(ctx ?? null);
				if (ctx) {
					const raw = localStorage.getItem(`git-sync-diff-viewer:${ctx.repo_path}`);
					if (raw) {
						try {
							const parsed = JSON.parse(raw) as {
								fontSize?: number;
								showSidebar?: boolean;
							};
							if (typeof parsed.fontSize === "number") {
								setFontSize(parsed.fontSize);
							}
							if (typeof parsed.showSidebar === "boolean") {
								setShowSidebar(parsed.showSidebar);
							}
						} catch {
							// Ignore invalid persisted state.
						}
					}
				}
			})
			.finally(() => {
				if (mounted) setIsContextLoading(false);
			});

		return () => {
			mounted = false;
		};
	}, []);

	useEffect(() => {
		localStorage.setItem(storageKey, JSON.stringify({ fontSize, showSidebar }));
	}, [fontSize, showSidebar, storageKey]);

	useEffect(() => {
		if (mode !== "compare_commits") {
			setLastLiveMode(mode);
		}
	}, [mode]);

	useEffect(() => {
		if (!context) return;
		if (mode === "compare_commits") return;

		const subscription = events.statusUpdateEvent.listen(
			(event: { payload: AppStatus }) => {
				const repoStatus = event.payload.repos.find(
					(repo) => repo.repo_path === context.repo_path,
				);
				if (!repoStatus || repoStatus.is_syncing || !repoStatus.last_sync_time) {
					return;
				}

				if (repoStatus.last_sync_time !== lastSyncTimeRef.current) {
					lastSyncTimeRef.current = repoStatus.last_sync_time;
					setLiveRefreshNotice(`Updated ${new Date(repoStatus.last_sync_time).toLocaleTimeString()}`);
					setRefreshTick((tick: number) => tick + 1);
				}
			},
		);

		return () => {
			subscription.then((dispose: () => void) => dispose());
		};
   }, [context, mode]);

	useEffect(() => {
		if (!context) return;

		let active = true;
		setError(null);

		commands.listDiffCommits(context.repo_path, 200).then((result) => {
			if (!active) return;
			if (result.status === "error") {
				setError(result.error);
				setCommits([]);
				return;
			}

			setCommits(result.data);
			if (result.data.length > 0) {
               setManualToSha((prev: string) => prev || result.data[0].sha);
				setManualFromSha(
                 (prev: string) =>
						prev || result.data[Math.min(1, result.data.length - 1)].sha,
				);
			}
		});

		return () => {
			active = false;
		};
	}, [context]);

	useEffect(() => {
		if (!context) return;
		if (mode === "compare_commits" && (!manualFromSha || !manualToSha)) return;

		let active = true;
		setIsViewLoading(true);
		setError(null);

		commands
			.getDiffView(
				context.repo_path,
				buildMode(mode, manualFromSha, manualToSha),
				selectedFile,
			)
			.then((result) => {
				if (!active) return;
				if (result.status === "error") {
					setError(result.error);
					setView(null);
					return;
				}

				setView(result.data);
             setSelectedFile((prev: string | null) => {
                 if (prev) {
						return prev;
					}
					return result.data.selected_file;
				});
			})
			.finally(() => {
				if (active) setIsViewLoading(false);
			});

		return () => {
			active = false;
		};
  }, [context, mode, manualFromSha, manualToSha, selectedFile, refreshTick]);

	useEffect(() => {
		if (!liveRefreshNotice) return;
		const timer = window.setTimeout(() => setLiveRefreshNotice(null), 2500);
		return () => window.clearTimeout(timer);
	}, [liveRefreshNotice]);

	useEffect(() => {
		const firstHunk = firstHunkRef.current;
		if (firstHunk) {
			firstHunk.scrollIntoView({ block: "start", behavior: "smooth" });
			return;
		}

		diffContainerRef.current?.scrollTo({ top: 0, behavior: "smooth" });
	}, [selectedFile, view?.diff_text]);

	const title = context?.repo_name
		? `${context.repo_name} Diff Viewer`
		: "Diff Viewer";

	const manualCompareDisabled = commits.length < 2;
	const liveBadge = mode === "compare_commits" ? "Pinned" : "Live";
  const diffLines = (() => {
		let markedFirstHunk = false;
       return (view?.diff_text ?? "").split("\n").map((text: string, index: number) => {
			const isFirstHunk = !markedFirstHunk && text.startsWith("@@");
			if (isFirstHunk) {
				markedFirstHunk = true;
			}

			let className = "text-foreground";
			if (text.startsWith("+++ ") || text.startsWith("--- ") || text.startsWith("@@")) {
				className = "text-muted-foreground";
			} else if (text.startsWith("+") && !text.startsWith("+++ ")) {
				className = "text-emerald-700 dark:text-emerald-400";
			} else if (text.startsWith("-") && !text.startsWith("--- ")) {
				className = "text-red-700 dark:text-red-400";
			}

			return {
				id: `${index}-${text}`,
				text,
				isFirstHunk,
				className,
			};
		});
	})();

	const handleFileSelect = (filePath: string) => {
		setSelectedFile(filePath);
		if (mode === "compare_commits") {
			setMode(lastLiveMode);
		}
	};

	const handleReturnToLive = () => {
		setMode(lastLiveMode);
	};

	return (
		<div className="bg-background text-foreground flex h-screen flex-col">
			<TitleBar inSettings={false} title={title} showStatusDot={false} />
			<div className="flex min-h-0 flex-1">
				{showSidebar && (
					<aside className="border-border bg-muted/20 flex w-80 shrink-0 flex-col border-r">
						<div className="border-border border-b px-4 py-3">
							<div className="text-sm font-semibold">Changed Files</div>
							<div className="text-muted-foreground mt-1 text-xs">
								Markdown files are shown first when available.
							</div>
						</div>
						<div className="min-h-0 flex-1 overflow-auto px-2 py-2">
							{view?.files.length ? (
								view.files.map((file: DiffViewData["files"][number]) => (
									<button
										key={file.path}
										type="button"
                                      onClick={() => handleFileSelect(file.path)}
										className={`hover:bg-muted flex w-full flex-col border px-3 py-2 text-left transition-colors ${
											selectedFile === file.path
												? "border-primary bg-primary/5"
												: "border-transparent"
										}`}
									>
										<span className="truncate text-sm font-medium">{file.path}</span>
										<span className="text-muted-foreground text-[11px] uppercase">
											{file.status}
										</span>
									</button>
								))
							) : (
								<div className="text-muted-foreground px-2 py-3 text-sm">
									No changed text files in this range.
								</div>
							)}
						</div>
					</aside>
				)}
				<main className="flex min-w-0 flex-1 flex-col">
					<div className="border-border flex flex-wrap items-center gap-2 border-b px-4 py-2.5">
						<select
							className="border-border bg-background h-7 min-w-40 border px-2 text-xs"
							value={mode}
                          onChange={(e: ChangeEvent<HTMLSelectElement>) => {
								setMode(e.target.value as ViewerMode);
								setSelectedFile(null);
							}}
						>
							<option value="latest_commit">Latest Commit</option>
							<option value="latest_author">Latest Author</option>
							<option value="compare_commits" disabled={manualCompareDisabled}>
								Compare Commits
							</option>
						</select>
						{mode === "compare_commits" && (
							<>
								<select
									className="border-border bg-background h-7 min-w-72 border px-2 text-xs"
									value={manualFromSha}
                                  onChange={(e: ChangeEvent<HTMLSelectElement>) => {
										setManualFromSha(e.target.value);
										setSelectedFile(null);
									}}
								>
									{commits.map((commit: DiffCommitSummary) => (
										<option key={`from-${commit.sha}`} value={commit.sha}>
											From: {formatCommitOption(commit)}
										</option>
									))}
								</select>
								<select
									className="border-border bg-background h-7 min-w-72 border px-2 text-xs"
									value={manualToSha}
                                  onChange={(e: ChangeEvent<HTMLSelectElement>) => {
										setManualToSha(e.target.value);
										setSelectedFile(null);
									}}
								>
									{commits.map((commit: DiffCommitSummary) => (
										<option key={`to-${commit.sha}`} value={commit.sha}>
											To: {formatCommitOption(commit)}
										</option>
									))}
								</select>
							</>
						)}
						<div className="ml-auto flex items-center gap-2">
                         {mode === "compare_commits" && (
								<Button variant="outline" size="sm" onClick={handleReturnToLive}>
									Return to Live
								</Button>
							)}
							<Button
								variant="ghost"
								size="sm"
								onClick={() => setShowSidebar((prev: boolean) => !prev)}
							>
								{showSidebar ? "Hide Files" : "Show Files"}
							</Button>
							<Button
								variant="ghost"
								size="sm"
								onClick={() => setFontSize((size: number) => Math.max(12, size - 1))}
							>
								A-
							</Button>
							<Button
								variant="ghost"
								size="sm"
								onClick={() => setFontSize(18)}
							>
								Reset
							</Button>
							<Button
								variant="ghost"
								size="sm"
								onClick={() => setFontSize((size: number) => Math.min(30, size + 1))}
							>
								A+
							</Button>
						</div>
					</div>
					<div className="border-border flex flex-wrap items-center gap-x-4 gap-y-1 border-b px-4 py-2 text-xs">
						<div className="text-muted-foreground truncate">
							{selectedFile ?? context?.repo_path ?? "Loading repository…"}
						</div>
						{view && (
							<>
								<div>Author: {view.range.author_name}</div>
								<div>{new Date(view.range.timestamp).toLocaleString()}</div>
								<div>From: {view.range.from_label}</div>
								<div>To: {view.range.to_label}</div>
                             {liveRefreshNotice && (
									<div className="text-muted-foreground">{liveRefreshNotice}</div>
								)}
							</>
						)}
						<div className="ml-auto bg-primary/10 text-primary rounded-full px-2 py-0.5 font-medium">
							{liveBadge}
						</div>
					</div>
                    <div
                        ref={(node: HTMLDivElement | null) => {
							diffContainerRef.current = node;
						}}
						className="min-h-0 flex-1 overflow-auto px-6 py-5"
					>
						{isContextLoading || isViewLoading ? (
							<div className="text-muted-foreground text-sm">Loading viewer…</div>
						) : error ? (
							<div className="text-sm text-red-600">{error}</div>
						) : view ? (
							view.diff_text ? (
                                <div
									className="bg-muted/20 border-border overflow-auto rounded-md border p-4 font-mono whitespace-pre-wrap"
									style={{ fontSize: `${fontSize}px`, lineHeight: 1.6 }}
								>
                                  {diffLines.map(
										(line: {
											id: string;
											text: string;
											isFirstHunk: boolean;
											className: string;
										}) => (
										<div
											key={line.id}
											ref={
												line.isFirstHunk
                                                   ? (node: HTMLDivElement | null) => {
														firstHunkRef.current = node;
													}
													: undefined
											}
											className={line.className}
										>
											{line.text || " "}
										</div>
                                 ),
									)}
								</div>
							) : (
								<div className="text-muted-foreground text-sm">
									No text diff is available for the current selection.
								</div>
							)
						) : (
							<div className="text-muted-foreground text-sm">
								This diff viewer window could not load its repository context.
							</div>
						)}
					</div>
				</main>
			</div>
		</div>
	);
}
