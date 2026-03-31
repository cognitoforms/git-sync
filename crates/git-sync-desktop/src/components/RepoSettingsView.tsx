import { useState } from "react";
import { FolderOpen } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { pickFolder } from "@/api";
import type React from "react";
import type { DesktopConfig, RepoConfig } from "@/types";

interface Props {
	config: DesktopConfig;
	idx: number | null;
	onSave: (newConfig: DesktopConfig) => void;
	onBack: () => void;
}

const DEFAULT_REPO: RepoConfig = {
	name: "",
	repo_path: "",
	remote: "origin",
	branch: "",
	interval_secs: 60,
	commit_message: "",
	sync_new_files: true,
	skip_hooks: false,
	conflict_branch: true,
};

export default function RepoSettingsView({
	config,
	idx,
	onSave,
	onBack,
}: Props) {
	const existing = idx !== null ? config.repositories[idx] : null;
	const [form, setForm] = useState<RepoConfig>(existing ?? DEFAULT_REPO);
	const [picking, setPicking] = useState(false);

	const set = (patch: Partial<RepoConfig>) =>
		setForm((f) => ({ ...f, ...patch }));

	const handleBrowse = async () => {
		setPicking(true);
		try {
			const path = await pickFolder();
			if (path) set({ repo_path: path });
		} finally {
			setPicking(false);
		}
	};

	const handleSave = () => {
		const repos = [...config.repositories];
		if (idx !== null) {
			repos[idx] = form;
		} else {
			repos.push(form);
		}
		onSave({ ...config, repositories: repos });
	};

	const handleDelete = () => {
		if (idx === null) return;
		onSave({
			...config,
			repositories: config.repositories.filter((_, i) => i !== idx),
		});
	};

	return (
		<div className="flex h-full flex-col">
			{/* Scrollable form */}
			<div className="flex-1 overflow-y-auto p-4">
				<div className="flex max-w-lg flex-col gap-4">
					<Field label="Display name (optional)">
						<Input
							value={form.name}
							onChange={(e) => set({ name: e.target.value })}
							placeholder="my-project"
						/>
					</Field>

					<Field label="Repository path">
						<div className="flex gap-2">
							<Input
								className="flex-1"
								value={form.repo_path}
								onChange={(e) => set({ repo_path: e.target.value })}
								placeholder="/path/to/repo"
							/>
							<Button
								variant="outline"
								size="sm"
								onClick={handleBrowse}
								disabled={picking}
								className="h-auto self-stretch"
							>
								<FolderOpen weight="bold" />
								{picking ? "…" : "Browse"}
							</Button>
						</div>
					</Field>

					<Field label="Remote">
						<Input
							value={form.remote}
							onChange={(e) => set({ remote: e.target.value })}
							placeholder="origin"
						/>
					</Field>

					<Field label="Branch (leave blank to auto-detect)">
						<Input
							value={form.branch}
							onChange={(e) => set({ branch: e.target.value })}
							placeholder="main"
						/>
					</Field>

					<Field label="Sync interval (seconds)">
						<Input
							type="number"
							min={10}
							value={form.interval_secs}
							onChange={(e) =>
								set({ interval_secs: Math.max(10, Number(e.target.value)) })
							}
							className="w-28"
						/>
					</Field>

					<Field label="Commit message (leave blank for default)">
						<Input
							value={form.commit_message}
							onChange={(e) => set({ commit_message: e.target.value })}
							placeholder="changes from {hostname} on {timestamp}"
						/>
					</Field>

					<div className="flex flex-col gap-2.5 pt-1">
						<CheckField
							label="Sync new (untracked) files"
							checked={form.sync_new_files}
							onChange={(v) => set({ sync_new_files: v })}
						/>
						<CheckField
							label="Skip git hooks on commit"
							checked={form.skip_hooks}
							onChange={(v) => set({ skip_hooks: v })}
						/>
						<CheckField
							label="Create conflict branch on merge conflict"
							checked={form.conflict_branch}
							onChange={(v) => set({ conflict_branch: v })}
						/>
					</div>
				</div>
			</div>

			{/* Footer */}
			<div className="border-border flex items-center justify-between border-t px-4 py-2.5">
				<div>
					{idx !== null && (
						<Button variant="destructive" size="sm" onClick={handleDelete}>
							Delete Repository
						</Button>
					)}
				</div>
				<div className="flex gap-2">
					<Button variant="ghost" size="sm" onClick={onBack}>
						Cancel
					</Button>
					<Button size="sm" onClick={handleSave} disabled={!form.repo_path}>
						Save
					</Button>
				</div>
			</div>
		</div>
	);
}

function Field({
	label,
	children,
}: {
	label: string;
	children: React.ReactNode;
}) {
	return (
		<div className="flex flex-col gap-1.5">
			<label className="text-foreground text-xs font-medium">{label}</label>
			{children}
		</div>
	);
}

function CheckField({
	label,
	checked,
	onChange,
}: {
	label: string;
	checked: boolean;
	onChange: (v: boolean) => void;
}) {
	return (
		<label className="text-foreground flex cursor-pointer items-center gap-2.5 text-xs select-none">
			<Checkbox
				checked={checked}
				onCheckedChange={(v) => onChange(v === true)}
			/>
			{label}
		</label>
	);
}
