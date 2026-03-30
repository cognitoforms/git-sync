import { X } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";

interface Props {
  onClose: () => void;
}

export default function AboutModal({ onClose }: Props) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={onClose}
    >
      <div
        className="bg-background border border-border w-72 p-6 flex flex-col items-center gap-3 shadow-lg"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex w-full justify-end -mt-2 -mr-2">
          <Button variant="ghost" size="icon-xs" onClick={onClose}>
            <X weight="bold" />
          </Button>
        </div>
        <div className="text-xl font-bold">git-sync</div>
        <div className="text-xs text-muted-foreground text-center">
          Automatic git repository synchronization
        </div>
        <Button size="sm" className="mt-2" onClick={onClose}>
          Close
        </Button>
      </div>
    </div>
  );
}
