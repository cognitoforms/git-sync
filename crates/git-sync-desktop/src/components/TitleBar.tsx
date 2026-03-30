import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, Minus, Square, X } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import StatusDot from "./StatusDot";

const appWindow = getCurrentWindow();

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
	return (
		<div
			className={cn(
				"border-border bg-background flex h-10 shrink-0 items-center border-b",
				className,
			)}
		>
			{/* Drag region — fills all space between left content and window controls */}
			<div
				className="flex h-full min-w-0 flex-1 items-center gap-2 overflow-hidden px-2"
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
					<StatusDot id={aggStatusId ?? "unknown"} className="shrink-0" />
				)}
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

			{/* Window controls */}
			<div className="flex h-full shrink-0">
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
			</div>
		</div>
	);
}
