import { ArrowCircleUp, ArrowsClockwise } from "@phosphor-icons/react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { useUpdater } from "@/hooks/useUpdater";

export default function UpdateBadge() {
	const { update, status, install } = useUpdater();

	if (!update) return null;

	return (
		<Button
			variant="ghost"
			size="icon-sm"
			onClick={() =>
			install().catch(() =>
				toast.error("Update failed", {
					description: "Could not install the update. Please try again later.",
				}),
			)
		}
			disabled={status === "installing"}
			className="text-primary h-full w-9 rounded-none"
			aria-label={`Update to ${update.version}`}
			title={`Update to ${update.version}`}
		>
			{status === "installing" ? (
				<ArrowsClockwise size={13} className="animate-spin" />
			) : (
				<ArrowCircleUp size={14} weight="fill" />
			)}
		</Button>
	);
}
