import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { ArrowLeft, Minus, Moon, Square, Sun, X } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useIsFullscreen } from "@/hooks/useIsFullscreen";
import { useTheme } from "./ThemeProvider";
import StatusDot from "./StatusDot";
import UpdateBadge from "./UpdateBadge";

const appWindow = getCurrentWindow();
const IS_MAC = platform() === "macos";

interface Props {
	inSettings: boolean;
	title: string;
	aggStatusId?: string;
	aggStatusLabel?: string;
	onBack?: () => void;
	className?: string;
}

export default function TitleBar({
	inSettings,
	title,
	aggStatusId,
	aggStatusLabel,
	onBack,
	className,
}: Props) {
	const { resolvedTheme, setTheme } = useTheme();
	const isFullscreen = useIsFullscreen();

	return (
		<div
			className={cn(
				"border-border bg-muted/50 flex h-10 shrink-0 items-center border-b",
				className,
			)}
		>
			{/* Drag region — fills all space between left content and window controls */}
			<div
				className={cn(
					"flex h-full min-w-0 flex-1 items-center gap-0.5 overflow-hidden",
					IS_MAC && !isFullscreen ? "pl-[82px]" : "px-2",
				)}
				data-tauri-drag-region
			>
				{inSettings ? (
					<Button
						variant="ghost"
						size="icon-sm"
						onClick={onBack}
						className="shrink-0"
					>
						<ArrowLeft weight="bold" />
					</Button>
				) : (
					<StatusDot
						id={aggStatusId ?? "unknown"}
						className="mr-2.5 ml-2 shrink-0"
					/>
				)}
				<div
					className="flex min-w-0 items-baseline gap-2 overflow-hidden"
					data-tauri-drag-region
				>
					<span
						className="truncate text-sm font-semibold select-none"
						data-tauri-drag-region
					>
						{title}
					</span>
					{!inSettings && aggStatusLabel && (
						<span
							className="text-muted-foreground truncate text-xs select-none"
							data-tauri-drag-region
						>
							{aggStatusLabel}
						</span>
					)}
				</div>
			</div>

			{/* Theme toggle + window controls */}
			<div className="flex h-full shrink-0 items-center">
				<UpdateBadge />
				<Button
					variant="ghost"
					size="icon-sm"
					onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}
					className="h-full w-9 rounded-none"
					aria-label="Toggle theme"
				>
					{resolvedTheme === "dark" ? <Sun size={13} /> : <Moon size={13} />}
				</Button>
				{!IS_MAC && (
					<>
						<button
							onClick={() => appWindow.minimize()}
							className="text-foreground/70 hover:bg-muted hover:text-foreground flex h-full w-10 items-center justify-center transition-colors"
							aria-label="Minimize"
						>
							<Minus size={12} weight="bold" />
						</button>
						<button
							onClick={() => appWindow.toggleMaximize()}
							className="text-foreground/70 hover:bg-muted hover:text-foreground flex h-full w-10 items-center justify-center transition-colors"
							aria-label="Maximize"
						>
							<Square size={11} />
						</button>
						<button
							onClick={() => appWindow.close()}
							className="text-foreground/70 flex h-full w-10 items-center justify-center transition-colors hover:bg-red-500 hover:text-white"
							aria-label="Close"
						>
							<X size={13} weight="bold" />
						</button>
					</>
				)}
			</div>
		</div>
	);
}
