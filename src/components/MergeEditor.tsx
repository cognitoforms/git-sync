import "@mismerge/core/styles.css";
import "@/styles/mismerge.css";
import { ComponentProps, useEffect, useRef } from "react";
import { MisMerge3 } from "@mismerge/react";
import { DefaultDarkColors, DefaultLightColors } from "@mismerge/react";
import { useTheme } from "./ThemeProvider";

type MisMerge3Instance = ComponentProps<typeof MisMerge3>;
type Props = Omit<
	React.ComponentProps<typeof MisMerge3>,
	"colors" | "className"
> & {
	className?: string;
};

export default function MergeEditor({ className, ...props }: Props) {
	const { resolvedTheme } = useTheme();
	const colors =
		resolvedTheme === "dark" ? DefaultDarkColors : DefaultLightColors;
	const wrapperRef = useRef<HTMLDivElement>(null);

	// always set colors on first render
	// workaround for https://github.com/BearToCode/mismerge/issues/25
	useEffect(() => {
		let timeoutId: ReturnType<typeof setTimeout> | null = null;
		let isMounted = true;

		const forceSetColors = () => {
			if (!isMounted) return;
			const misMergeEl = wrapperRef.current?.firstElementChild as
				| MisMerge3Instance
				| undefined;

			if (misMergeEl) {
				misMergeEl.colors = { ...colors };
			} else {
				timeoutId = setTimeout(forceSetColors, 0);
			}
		};
		forceSetColors();

		return () => {
			isMounted = false;
			if (timeoutId !== null) clearTimeout(timeoutId);
		};
	}, [colors]);

	return (
		<div ref={wrapperRef} className={className}>
			<MisMerge3 {...props} colors={colors} className="h-full!" />
		</div>
	);
}
