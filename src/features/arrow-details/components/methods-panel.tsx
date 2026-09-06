import { useState, type JSX } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Frame, FrameHeader, FramePanel, FrameTitle } from '@/components/ui/frame';

import type { ArrowLifecycle, ArrowMethod, ArrowStepDefinition, ArrowVariable } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

import { InfoIcon } from 'lucide-react';

import { StepPreviewModal } from './step-preview-modal';
import type { ArrowActionLabelKey } from '../lib/actions';

interface MethodsPanelProps {
	/** The five reserved lifecycle actions, listed ahead of the custom methods below. Omitted entirely when the caller has no target to read one from. */
	lifecycle?: ArrowLifecycle;
	methods: ArrowMethod[];
	variables: ArrowVariable[];
	values?: Record<string, string>;
	onValueChange?: (name: string, value: string) => void;
}

/** A method row's shared shape, whichever of the two sources below it came from. */
interface MethodRow {
	key: string;
	name: string;
	description?: string;
	steps: ArrowStepDefinition[];
}

/**
 * Lifecycle actions carry no name/description on the wire (unlike a custom
 * `ArrowMethod`) -- these are the same labels already shown on the Hero's
 * action buttons for each, kept in the order they'd actually run in an
 * arrow's life.
 */
const LIFECYCLE_ACTIONS: { key: keyof ArrowLifecycle; labelKey: ArrowActionLabelKey }[] = [
	{ key: 'install', labelKey: 'arrow.action.install' },
	{ key: 'update', labelKey: 'arrow.action.update' },
	{ key: 'execute', labelKey: 'arrow.action.start' },
	{ key: 'stop', labelKey: 'arrow.action.stop' },
	{ key: 'uninstall', labelKey: 'arrow.action.uninstall' },
];

/**
 * Read-only documentation of what an arrow can do -- the five reserved
 * lifecycle actions (install/update/execute/stop/uninstall) followed by
 * every custom method the manifest declares (e.g. "backup", "rcon"), all
 * deliberately not actionable. There's no way to run either from the app
 * today (a real gap, tracked separately); this panel exists to show what
 * exists and what it does, not to fix that here. A lifecycle action with no
 * declared steps is left out rather than shown as "0 steps": an empty step
 * list means core has nothing declared for it, not that there's a method
 * worth documenting -- unlike a custom method, which is shown regardless,
 * since it was named in the manifest either way.
 */
export function MethodsPanel({ lifecycle, methods, variables, values, onValueChange }: MethodsPanelProps): JSX.Element {
	const { t } = useTranslation();
	const [previewing, setPreviewing] = useState<MethodRow | null>(null);

	const lifecycleRows: MethodRow[] = lifecycle
		? LIFECYCLE_ACTIONS.filter(({ key }) => lifecycle[key].length > 0).map(({ key, labelKey }) => ({
				key,
				name: t(labelKey),
				steps: lifecycle[key],
			}))
		: [];
	const methodRows: MethodRow[] = methods.map((method) => ({
		key: method.name,
		name: method.name,
		description: method.description,
		steps: method.steps,
	}));
	const rows = [...lifecycleRows, ...methodRows];

	return (
		<Frame>
			<FrameHeader>
				<FrameTitle className="flex items-center gap-2">
					{t('arrow.tab.methods')}
					<Badge size="sm" variant="secondary">
						{t('arrow.methods.count', { count: rows.length })}
					</Badge>
				</FrameTitle>
			</FrameHeader>
			<FramePanel>
				<div className="flex flex-col divide-y divide-border">
					{rows.map((row) => (
						<div className="flex items-center gap-3 py-3 first:pt-0 last:pb-0" key={row.key}>
							<div className="min-w-0 flex-1">
								<p className="truncate font-mono text-sm font-medium">{row.name}</p>
								{row.description && (
									<p className="mt-0.5 truncate text-xs text-muted-foreground">{row.description}</p>
								)}
								<p className="mt-0.5 text-xs text-muted-foreground">{`${row.steps.length} steps`}</p>
							</div>
							<Button
								aria-label={t('arrow.action.info')}
								onClick={() => setPreviewing(row)}
								size="icon-sm"
								variant="ghost"
							>
								<InfoIcon aria-hidden="true" />
							</Button>
						</div>
					))}
				</div>
			</FramePanel>
			<StepPreviewModal
				// The dialog is fully controlled from here (no DialogTrigger of its
				// own) -- Base UI only ever calls this to report a close (Escape,
				// backdrop click, the close button), never to request an open, so
				// clearing the selection unconditionally is correct, not a shortcut.
				onOpenChange={() => setPreviewing(null)}
				onValueChange={onValueChange}
				open={previewing !== null}
				steps={previewing?.steps ?? []}
				title={previewing?.name ?? ''}
				values={values}
				variables={variables}
			/>
		</Frame>
	);
}
