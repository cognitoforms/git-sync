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
				className="bg-background border-border flex w-72 flex-col items-center gap-3 border p-6 shadow-lg"
				onClick={(e) => e.stopPropagation()}
			>
				<div className="-mt-2 -mr-2 flex w-full justify-end">
					<Button variant="ghost" size="icon-xs" onClick={onClose}>
						<X weight="bold" />
					</Button>
				</div>
				<div className="text-xl font-bold">git-sync</div>
				<div className="text-muted-foreground text-center text-xs">
					Automatic git repository synchronization
				</div>
				<Button size="sm" className="mt-2" onClick={onClose}>
					Close
				</Button>
			</div>
		</div>
	);
}
