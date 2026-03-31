import { cn } from "@/lib/utils";

const STATUS_COLOR: Record<string, string> = {
  equal: "bg-green-500",
  ahead: "bg-amber-400",
  behind: "bg-amber-400",
  diverged: "bg-red-500",
  "no-upstream": "bg-muted-foreground/60",
  syncing: "bg-blue-500",
  "error-warning": "bg-amber-500",
  "error-critical": "bg-red-500",
  unknown: "bg-muted-foreground/40",
};

interface Props {
  id: string;
  syncing?: boolean;
  errorLevel?: "warning" | "critical";
  className?: string;
}

export default function StatusDot({ id, syncing, errorLevel, className }: Props) {
  const effectiveId = syncing
    ? "syncing"
    : errorLevel
      ? `error-${errorLevel}`
      : id;
  const colorClass = STATUS_COLOR[effectiveId] ?? "bg-muted-foreground/40";
  return (
    <span
      className={cn("inline-block size-2 rounded-full shrink-0", colorClass, className)}
      aria-label={effectiveId}
    />
  );
}
