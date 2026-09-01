import { useState, type JSX } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Frame, FrameHeader, FramePanel, FrameTitle } from '@/components/ui/frame';

import type { ArrowMethod, ArrowVariable } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

import { InfoIcon } from 'lucide-react';

import { StepPreviewModal } from './step-preview-modal';

interface MethodsPanelProps {
	methods: ArrowMethod[];
	variables: ArrowVariable[];
	values?: Record<string, string>;
	onValueChange?: (name: string, value: string) => void;
}

/**
 * Read-only documentation of every custom method a manifest declares (e.g.
 * "backup", "rcon") -- deliberately not actionable. There's no way to run a
 * method from the app today (a real gap, tracked separately); this panel
 * exists to show what exists and what it does, not to fix that here.
 */
export function MethodsPanel({ methods, variables, values, onValueChange }: MethodsPanelProps): JSX.Element {
	const { t } = useTranslation();
	const [previewing, setPreviewing] = useState<ArrowMethod | null>(null);

	return (
		<Frame>
			<FrameHeader>
				<FrameTitle className="flex items-center gap-2">
					{t('arrow.tab.methods')}
					<Badge size="sm" variant="secondary">
						{t('arrow.methods.count', { count: methods.length })}
					</Badge>
				</FrameTitle>
			</FrameHeader>
			<FramePanel>
				<div className="flex flex-col divide-y divide-border">
					{methods.map((method) => (
						<div className="flex items-center gap-3 py-3 first:pt-0 last:pb-0" key={method.name}>
							<div className="min-w-0 flex-1">
								<p className="truncate font-mono text-sm font-medium">{method.name}</p>
								{method.description && (
									<p className="mt-0.5 truncate text-xs text-muted-foreground">
										{method.description}
									</p>
								)}
								<p className="mt-0.5 text-xs text-muted-foreground">{`${method.steps.length} steps`}</p>
							</div>
							<Button
								aria-label={t('arrow.action.info')}
								onClick={() => setPreviewing(method)}
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
