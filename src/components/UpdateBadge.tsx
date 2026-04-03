import { ArrowsClockwise, DownloadSimple, X } from "@phosphor-icons/react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useUpdater } from "@/hooks/useUpdater";
import { Separator } from "./ui/separator";

export default function UpdateBadge() {
	const { update, status, dismissed, install, dismiss } = useUpdater();

	if (!update || dismissed) return null;

	const installing = status === "installing";

	return (
		<Badge variant="outline" className="mx-1 h-auto gap-0 p-0">
			<Button
				variant="ghost"
				size="xs"
				onClick={() =>
					install().catch(() =>
						toast.error("Update failed", {
							description:
								"Could not install the update. Please try again later.",
						}),
					)
				}
				disabled={installing}
				className="font-normal"
				aria-label={`Update to v${update.version}`}
				title={`Restart to install v${update.version}`}
			>
				{installing ? (
					<ArrowsClockwise size={12} className="animate-spin" />
				) : (
					<DownloadSimple size={12} weight="bold" />
				)}
				Update to v{update.version}
			</Button>

			<Separator orientation="vertical" />

			<Button
				variant="ghost"
				size="icon-xs"
				onClick={dismiss}
				disabled={installing}
				aria-label="Dismiss update"
			>
				<X size={10} weight="bold" />
			</Button>
		</Badge>
	);
}
