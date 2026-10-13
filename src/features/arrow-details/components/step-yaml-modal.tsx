import type { JSX } from 'react';

import { Dialog, DialogDescription, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from '@/components/ui/dialog';

import type { Overridable } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

import { Code2Icon } from 'lucide-react';

/**
 * Deliberately a bare `type: string`, not `StepType` -- this modal inspects
 * both not-yet-run manifest steps (`ArrowStepDefinition`, `type: StepType`)
 * and live/historical `StepProgress` rows from the Activity tab, whose
 * `type` comes straight off the wire and is not guaranteed to be one of the
 * four known `StepType` values (real mock fixture data already emits
 * `'exec'`). Every field past `type`/`title` is optional and only ever
 * present on a manifest step -- `StepProgress` genuinely carries nothing
 * else (verified against core's own `StepProgressDTO`), so a `StepProgress`
 * row here just renders its `type`/`title` and nothing more, honestly.
 */
export interface StepYamlStep {
	type: string;
	title: string;
	exit_on_failure?: boolean;
	command?: Overridable<string>;
	elevated?: Overridable<boolean>;
	url?: Overridable<string>;
	to?: Overridable<string>;
	checksum?: Overridable<string>;
	signal?: Overridable<string>;
	timeout?: Overridable<string>;
}

interface StepYamlModalProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	step: StepYamlStep | null;
}

// Declaration order for whichever of these are actually present on the step
// -- `type`/`title` always come first, unconditionally, ahead of this list.
const RAW_FIELDS: ReadonlyArray<Exclude<keyof StepYamlStep, 'type' | 'title'>> = [
	'exit_on_failure',
	'command',
	'elevated',
	'url',
	'to',
	'checksum',
	'signal',
	'timeout',
];

function formatValue(value: unknown): string {
	if (typeof value === 'string') return value === '' ? '""' : value;
	if (value !== null && typeof value === 'object') return JSON.stringify(value);
	return String(value);
}

/** The step's raw fields, one per line -- only those actually present (manifest steps carry them; a `StepProgress` row never does). */
function toYaml(step: StepYamlStep): string {
	const lines = [`type: ${step.type}`, `title: ${step.title}`];
	for (const field of RAW_FIELDS) {
		const value = step[field];
		if (value !== undefined) lines.push(`${field}: ${formatValue(value)}`);
	}
	return lines.join('\n');
}

/**
 * A small, technical-user-only "inspect this step" dialog -- the step's full
 * raw definition (command on a run step, url on a fetch step, signal kind on
 * a signal step, and so on), not just its type and title.
 */
export function StepYamlModal({ open, onOpenChange, step }: StepYamlModalProps): JSX.Element {
	const { t } = useTranslation();

	return (
		<Dialog onOpenChange={onOpenChange} open={open}>
			<DialogPopup>
				{step && (
					<>
						<DialogHeader>
							<div className="flex items-center gap-2">
								<Code2Icon aria-hidden="true" className="size-4.5 shrink-0 text-muted-foreground" />
								<DialogTitle>{step.title}</DialogTitle>
							</div>
							<DialogDescription>{t('arrow.step.modal.title')}</DialogDescription>
						</DialogHeader>
						<DialogPanel>
							<pre className="whitespace-pre-wrap break-words rounded-md bg-muted p-3 font-mono text-xs text-foreground">
								{toYaml(step)}
							</pre>
						</DialogPanel>
					</>
				)}
			</DialogPopup>
		</Dialog>
	);
}
