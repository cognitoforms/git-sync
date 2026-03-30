import { cn } from "@/lib/utils";

const STATUS_COLOR: Record<string, string> = {
  equal: "bg-green-500",
  ahead: "bg-amber-400",
  behind: "bg-amber-400",
  diverged: "bg-red-500",
  "no-upstream": "bg-muted-foreground/60",
  syncing: "bg-blue-500",
  unknown: "bg-muted-foreground/40",
};

interface Props {
  id: string;
  syncing?: boolean;
  className?: string;
}

export default function StatusDot({ id, syncing, className }: Props) {
  const effectiveId = syncing ? "syncing" : id;
  const colorClass = STATUS_COLOR[effectiveId] ?? "bg-muted-foreground/40";
  return (
    <span
      className={cn("inline-block size-2 rounded-full shrink-0", colorClass, className)}
      aria-label={effectiveId}
    />
  );
}
