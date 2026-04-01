import { useForm, Controller } from "react-hook-form";
import { toast } from "sonner";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel, FieldError } from "@/components/ui/field";
import { type ResolvedConfig } from "@/hooks/queries";

interface Props {
	config: ResolvedConfig;
	onSave: (newConfig: ResolvedConfig) => void;
	onBack: () => void;
}

const schema = z.object({
	remote: z.string().min(1, "Remote is required"),
	interval_secs: z.coerce.number().int().min(10, "Minimum 10 seconds"),
	commit_message: z.string(),
	debounce_ms: z.coerce.number().int().min(0, "Minimum 0"),
	sync_new_files: z.boolean(),
	skip_hooks: z.boolean(),
	conflict_branch: z.boolean(),
	sync_on_start: z.boolean(),
});

type FormValues = z.infer<typeof schema>;

export default function GlobalSettingsView({ config, onSave, onBack }: Props) {
	const {
		control,
		handleSubmit,
		formState: { errors, isValid },
	} = useForm<FormValues>({
		resolver: standardSchemaResolver(schema),
		defaultValues: config.global,
		mode: "onBlur",
	});

	const onSubmit = (values: FormValues) => {
		onSave({ ...config, global: values });
		toast.success("Global settings saved");
	};

	return (
		<div className="flex h-full flex-col">
			<div className="flex-1 overflow-y-auto p-4">
				<form id="global-settings-form" onSubmit={handleSubmit(onSubmit)}>
					<div className="flex max-w-lg flex-col gap-4">
						<Field data-invalid={!!errors.remote}>
							<FieldLabel>Default remote</FieldLabel>
							<Controller
								name="remote"
								control={control}
								render={({ field }) => (
									<Input {...field} placeholder="origin" />
								)}
							/>
							<FieldError errors={[errors.remote]} />
						</Field>

						<Field data-invalid={!!errors.interval_secs}>
							<FieldLabel>Default sync interval (seconds)</FieldLabel>
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
							<FieldLabel>Default file change debounce (ms)</FieldLabel>
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
							<FieldLabel>
								Default commit message (leave blank for built-in default)
							</FieldLabel>
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
								[
									"sync_new_files",
									"skip_hooks",
									"conflict_branch",
									"sync_on_start",
								] as const
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
			<div className="border-border flex items-center justify-end gap-2 border-t px-4 py-2.5">
				<Button variant="ghost" size="sm" onClick={onBack}>
					Cancel
				</Button>
				<Button
					type="submit"
					form="global-settings-form"
					size="sm"
					disabled={!isValid}
				>
					Save
				</Button>
			</div>
		</div>
	);
}
