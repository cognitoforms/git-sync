import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { ArrowLeft, Minus, Moon, Square, Sun, X } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useIsFullscreen } from "@/hooks/useIsFullscreen";
import { useTheme } from "./ThemeProvider";
import StatusDot from "./StatusDot";

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
        "flex h-10 shrink-0 items-center border-b border-border bg-muted/50",
        className,
      )}
    >
      {/* Drag region — fills all space between left content and window controls */}
      <div
        className={cn(
          "flex flex-1 items-center gap-0.5 h-full min-w-0 overflow-hidden",
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
            className="shrink-0 ml-2 mr-2.5"
          />
        )}
        <div
          className="flex items-baseline gap-2 min-w-0 overflow-hidden"
          data-tauri-drag-region
        >
          <span
            className="text-sm font-semibold truncate select-none"
            data-tauri-drag-region
          >
            {title}
          </span>
          {!inSettings && aggStatusLabel && (
            <span
              className="text-xs text-muted-foreground truncate select-none"
              data-tauri-drag-region
            >
              {aggStatusLabel}
            </span>
          )}
        </div>
      </div>

      {/* Theme toggle + window controls */}
      <div className="flex h-full shrink-0">
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}
          className="rounded-none h-full w-9"
          aria-label="Toggle theme"
        >
          {resolvedTheme === "dark" ? <Sun size={13} /> : <Moon size={13} />}
        </Button>
        {!IS_MAC && (
          <>
            <button
              onClick={() => appWindow.minimize()}
              className="flex items-center justify-center w-10 h-full text-foreground/70 hover:bg-muted hover:text-foreground transition-colors"
              aria-label="Minimize"
            >
              <Minus size={12} weight="bold" />
            </button>
            <button
              onClick={() => appWindow.toggleMaximize()}
              className="flex items-center justify-center w-10 h-full text-foreground/70 hover:bg-muted hover:text-foreground transition-colors"
              aria-label="Maximize"
            >
              <Square size={11} />
            </button>
            <button
              onClick={() => appWindow.close()}
              className="flex items-center justify-center w-10 h-full text-foreground/70 hover:bg-red-500 hover:text-white transition-colors"
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
