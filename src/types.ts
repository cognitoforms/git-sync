export type View =
	| { kind: "list" }
	| { kind: "settings"; idx: number | null }
	| { kind: "global-settings" }
	| { kind: "about" };
