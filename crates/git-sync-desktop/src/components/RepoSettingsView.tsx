import { useForm, Controller } from "react-hook-form";
import { toast } from "sonner";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { z } from "zod";
import { FolderOpen } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel, FieldError } from "@/components/ui/field";
import { pickFolder, validateRepoPath } from "@/api";
import type { DesktopConfig, GlobalSettings } from "@/types";

interface Props {
	config: DesktopConfig;
	idx: number | null;
	globalSettings: GlobalSettings;
	onSave: (newConfig: DesktopConfig) => void;
	onBack: () => void;
}

const schema = z.object({
	name: z.string(),
	repo_path: z.string().min(1, "Repository path is required"),
	remote: z.string().min(1, "Remote is required"),
	branch: z.string(),
	interval_secs: z.coerce.number().int().min(10, "Minimum 10 seconds"),
	commit_message: z.string(),
	sync_new_files: z.boolean(),
	skip_hooks: z.boolean(),
	conflict_branch: z.boolean(),
	sync_on_start: z.boolean(),
	debounce_ms: z.coerce.number().int().min(0, "Minimum 0"),
});

type FormValues = z.infer<typeof schema>;

export default function RepoSettingsView({
	config,
	idx,
	globalSettings,
	onSave,
	onBack,
}: Props) {
	const existing = idx !== null ? config.repositories[idx] : null;

	const defaultValues: FormValues = {
		name: "",
		repo_path: "",
		remote: globalSettings.remote,
		branch: "",
		interval_secs: globalSettings.interval_secs,
		commit_message: globalSettings.commit_message,
		sync_new_files: globalSettings.sync_new_files,
		skip_hooks: globalSettings.skip_hooks,
		conflict_branch: globalSettings.conflict_branch,
		sync_on_start: globalSettings.sync_on_start,
		debounce_ms: globalSettings.debounce_ms,
	};

	const {
		control,
		handleSubmit,
		setError,
		clearErrors,
		formState: { errors, isValid },
	} = useForm<FormValues>({
		resolver: standardSchemaResolver(schema),
		defaultValues: existing ?? defaultValues,
		mode: "onBlur",
	});

	const checkGitRepo = async (path: string) => {
		if (!path) return;
		const valid = await validateRepoPath(path);
		if (!valid) {
			setError("repo_path", { message: "Directory is not a Git repository" });
		} else {
			clearErrors("repo_path");
		}
	};

	const onSubmit = (values: FormValues) => {
		const repos = [...config.repositories];
		if (idx !== null) {
			repos[idx] = values;
		} else {
			repos.push(values);
		}
		onSave({ ...config, repositories: repos });
		toast.success("Repository settings saved");
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
				<form id="repo-settings-form" onSubmit={handleSubmit(onSubmit)}>
					<div className="flex max-w-lg flex-col gap-4">
						<Field>
							<FieldLabel>Display name (optional)</FieldLabel>
							<Controller
								name="name"
								control={control}
								render={({ field }) => (
									<Input {...field} placeholder="my-project" />
								)}
							/>
						</Field>

						<Field data-invalid={!!errors.repo_path}>
							<FieldLabel>Repository path</FieldLabel>
							<Controller
								name="repo_path"
								control={control}
								render={({ field }) => (
									<div className="flex gap-2">
										<Input
											{...field}
											className="flex-1"
											placeholder="/path/to/repo"
											onBlur={() => checkGitRepo(field.value)}
										/>
										<Button
											type="button"
											variant="outline"
											size="sm"
											onClick={async () => {
												const path = await pickFolder();
												if (path) {
													field.onChange(path);
													await checkGitRepo(path);
												}
											}}
											className="h-auto self-stretch"
										>
											<FolderOpen weight="bold" />
											Browse
										</Button>
									</div>
								)}
							/>
							<FieldError errors={[errors.repo_path]} />
						</Field>

						<Field data-invalid={!!errors.remote}>
							<FieldLabel>Remote</FieldLabel>
							<Controller
								name="remote"
								control={control}
								render={({ field }) => (
									<Input {...field} placeholder="origin" />
								)}
							/>
							<FieldError errors={[errors.remote]} />
						</Field>

						<Field>
							<FieldLabel>Branch (leave blank to auto-detect)</FieldLabel>
							<Controller
								name="branch"
								control={control}
								render={({ field }) => <Input {...field} placeholder="main" />}
							/>
						</Field>

						<Field data-invalid={!!errors.interval_secs}>
							<FieldLabel>Sync interval (seconds)</FieldLabel>
							<Controller
								name="interval_secs"
								control={control}
								render={({ field }) => (
									<Input type="number" min={10} {...field} className="w-28" />
								)}
							/>
							<FieldError errors={[errors.interval_secs]} />
						</Field>

						<Field data-invalid={!!errors.debounce_ms}>
							<FieldLabel>File change debounce (ms)</FieldLabel>
							<Controller
								name="debounce_ms"
								control={control}
								render={({ field }) => (
									<Input type="number" min={0} {...field} className="w-28" />
								)}
							/>
							<FieldError errors={[errors.debounce_ms]} />
						</Field>

						<Field>
							<FieldLabel>Commit message (leave blank for default)</FieldLabel>
							<Controller
								name="commit_message"
								control={control}
								render={({ field }) => (
									<Input
										{...field}
										placeholder="changes from {hostname} on {timestamp}"
									/>
								)}
							/>
						</Field>

						<div className="flex flex-col gap-2.5 pt-1">
							{(
								["sync_new_files", "skip_hooks", "conflict_branch", "sync_on_start"] as const
							).map((name) => (
								<Controller
									key={name}
									name={name}
									control={control}
									render={({ field }) => (
										<label className="text-foreground flex cursor-pointer items-center gap-2.5 text-xs select-none">
											<Checkbox
												checked={field.value}
												onCheckedChange={(v) => field.onChange(v === true)}
											/>
											{
												{
													sync_new_files: "Sync new (untracked) files",
													skip_hooks: "Skip git hooks on commit",
													conflict_branch:
														"Create conflict branch on merge conflict",
													sync_on_start: "Sync when app starts",
												}[name]
											}
										</label>
									)}
								/>
							))}
						</div>
					</div>
				</form>
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
					<Button
						type="submit"
						form="repo-settings-form"
						size="sm"
						disabled={!isValid}
					>
						Save
					</Button>
				</div>
			</div>
		</div>
	);
}
