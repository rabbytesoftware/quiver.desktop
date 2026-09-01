import { useState, type JSX } from 'react';

import { Button } from '@/components/ui/button';
import { Dialog, DialogDescription, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from '@/components/ui/dialog';

import type { ArrowStepDefinition, ArrowVariable, StepType } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

import { BoxIcon, DownloadIcon, InfoIcon, ListIcon, RadioIcon, type LucideIcon } from 'lucide-react';

import { StepYamlModal } from './step-yaml-modal';
import { VariablesSettingsModal } from './variables-settings-modal';

interface StepPreviewModalProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	/** e.g. "Install", "Start", or a custom method's own name. */
	title: string;
	steps: ArrowStepDefinition[];
	/** Variable names this action consumes, e.g. `['server-name', 'difficulty']`. */
	usesVariables?: string[];
	/** Full variable definitions, passed through to the "Configure" link. */
	variables?: ArrowVariable[];
	values?: Record<string, string>;
	onValueChange?: (name: string, value: string) => void;
}

const STEP_ICONS: Record<StepType, LucideIcon> = {
	dependencies: BoxIcon,
	fetch: DownloadIcon,
	run: ListIcon,
	signal: RadioIcon,
};

function noop(): void {
	/* onValueChange is optional; this keeps the modal usable without one. */
}

/**
 * Read-only "what will run" preview -- each step's kind and name, plus (via
 * its own "inspect" trigger) the full raw definition core actually declared
 * for it: the command on a run step, the url on a fetch step, and so on.
 */
export function StepPreviewModal({
	open,
	onOpenChange,
	title,
	steps,
	usesVariables,
	variables,
	values,
	onValueChange,
}: StepPreviewModalProps): JSX.Element {
	const { t } = useTranslation();
	const uses = usesVariables ?? [];
	const [inspecting, setInspecting] = useState<ArrowStepDefinition | null>(null);

	return (
		<Dialog onOpenChange={onOpenChange} open={open}>
			<DialogPopup>
				<DialogHeader>
					<DialogTitle>{title}</DialogTitle>
					<DialogDescription>{t('arrow.preview.subtitle')}</DialogDescription>
				</DialogHeader>
				<DialogPanel>
					<ol className="flex flex-col gap-2">
						{steps.map((step) => {
							const Icon = STEP_ICONS[step.type];
							return (
								<li
									className="flex items-center gap-2.5 rounded-md border px-3 py-2"
									key={`${step.type}:${step.title}`}
								>
									<Icon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
									<span className="min-w-0 flex-1 truncate text-sm">{step.title}</span>
									<span className="shrink-0 text-xs text-muted-foreground">
										{t(`arrow.step.type.${step.type}`)}
									</span>
									<Button
										aria-label={t('arrow.step.inspect')}
										onClick={() => setInspecting(step)}
										size="icon-xs"
										variant="ghost"
									>
										<InfoIcon aria-hidden="true" />
									</Button>
								</li>
							);
						})}
					</ol>

					{uses.length > 0 && (
						<div className="mt-4 flex flex-col gap-2 border-t pt-4">
							<p className="text-xs text-muted-foreground">
								<span className="font-medium text-foreground">{t('arrow.preview.uses')}</span>
								{': '}
								{uses.join(', ')}
							</p>
							<VariablesSettingsModal
								onChange={onValueChange ?? noop}
								values={values ?? {}}
								variables={variables ?? []}
							/>
						</div>
					)}
				</DialogPanel>
			</DialogPopup>

			<StepYamlModal
				// Fully controlled from here (no DialogTrigger of its own) -- Base UI
				// only ever calls this to report a close, never to request an open,
				// so clearing the selection unconditionally is correct, not a shortcut.
				onOpenChange={() => setInspecting(null)}
				open={inspecting !== null}
				step={inspecting}
			/>
		</Dialog>
	);
}
