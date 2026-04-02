import { ArrowCircleUp, ArrowsClockwise } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { useUpdater } from "@/hooks/useUpdater";

export default function UpdateBadge() {
	const { update, status, install } = useUpdater();

	if (!update) return null;

	return (
		<Button
			variant="ghost"
			size="icon-sm"
			onClick={install}
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
