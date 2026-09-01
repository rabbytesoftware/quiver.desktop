import { useState, type JSX } from 'react';

import { Button } from '@/components/ui/button';
import { FlickerSpinner } from '@/components/ui/flicker-spinner';
import { Frame, FrameDescription, FrameHeader, FramePanel, FrameTitle } from '@/components/ui/frame';

import type { ActiveRun, LastReturn, StepProgress, StepType } from '@/domain/arrow';
import { cn } from '@/lib/cn';
import { useTranslation, type Translator } from '@/lib/i18n';

import {
	BoxIcon,
	CheckIcon,
	CircleHelpIcon,
	DownloadIcon,
	InfoIcon,
	ListIcon,
	RadioIcon,
	SquareIcon,
	XIcon,
	type LucideIcon,
} from 'lucide-react';

import { StepYamlModal, type StepYamlStep } from './step-yaml-modal';

interface StepsTimelineProps {
	activeRun: ActiveRun | null;
	lastReturn: LastReturn | null;
	/** When false, this is a Discovered arrow -- the empty state explains why there's nothing to show instead of just saying there's nothing yet. */
	userInstalled: boolean;
}

/**
 * Only the five reserved lifecycle verbs get a friendly label -- everything
 * else (a custom method name, e.g. "backup") has no `arrow.action.*` entry to
 * look up, so it's shown as-is. `execute` resolves to the same "Start" label
 * the Hero uses for it, not a separate "Execute" string.
 */
function methodLabel(t: Translator['t'], method: string): string {
	switch (method) {
		case 'install':
			return t('arrow.action.install');
		case 'uninstall':
			return t('arrow.action.uninstall');
		case 'update':
			return t('arrow.action.update');
		case 'execute':
			return t('arrow.action.start');
		case 'stop':
			return t('arrow.action.stop');
		default:
			return method;
	}
}

const OUTCOME_ICONS: Record<LastReturn['outcome'], LucideIcon> = {
	cancelled: SquareIcon,
	failed: XIcon,
	success: CheckIcon,
};

function outcomeLabel(t: Translator['t'], outcome: LastReturn['outcome']): string {
	switch (outcome) {
		case 'success':
			return t('arrow.activity.outcome.success');
		case 'failed':
			return t('arrow.activity.outcome.failed');
		case 'cancelled':
			return t('arrow.activity.outcome.cancelled');
	}
}

const KNOWN_STEP_TYPES: readonly StepType[] = ['run', 'fetch', 'signal', 'dependencies'];

const STEP_TYPE_ICONS: Record<StepType, LucideIcon> = {
	dependencies: BoxIcon,
	fetch: DownloadIcon,
	run: ListIcon,
	signal: RadioIcon,
};

function isKnownStepType(type: string): type is StepType {
	return (KNOWN_STEP_TYPES as readonly string[]).includes(type);
}

/**
 * `step.type` is a bare string off the wire -- real fixture data already uses
 * values like `'exec'` outside the four known kinds, so an unrecognised type
 * falls back to a generic icon rather than throwing or rendering nothing.
 */
function iconForStepType(type: string): LucideIcon {
	return isKnownStepType(type) ? STEP_TYPE_ICONS[type] : CircleHelpIcon;
}

interface StepStatusDotProps {
	status: StepProgress['status'];
	displayIndex: number;
}

function StepStatusDot({ status, displayIndex }: StepStatusDotProps): JSX.Element {
	if (status === 'completed') {
		return (
			<span className="flex size-6 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground">
				<CheckIcon aria-hidden="true" className="size-3.5" />
			</span>
		);
	}

	if (status === 'failed') {
		return (
			<span className="flex size-6 shrink-0 items-center justify-center rounded-full bg-destructive text-white">
				<XIcon aria-hidden="true" className="size-3.5" />
			</span>
		);
	}

	if (status === 'running') {
		return (
			<span className="flex size-6 shrink-0 items-center justify-center rounded-full border border-border">
				<FlickerSpinner />
			</span>
		);
	}

	return (
		<span className="flex size-6 shrink-0 items-center justify-center rounded-full border border-border text-xs font-medium text-muted-foreground">
			{displayIndex}
		</span>
	);
}

interface StepRowProps {
	step: StepProgress;
	isLast: boolean;
	inspectLabel: string;
	onInspect: (step: StepYamlStep) => void;
}

function StepRow({ step, isLast, inspectLabel, onInspect }: StepRowProps): JSX.Element {
	const TypeIcon = iconForStepType(step.type);

	return (
		<li className="relative flex gap-3">
			<div className="flex flex-col items-center">
				<StepStatusDot displayIndex={step.index + 1} status={step.status} />
				{!isLast && <span aria-hidden="true" className="my-1 w-px flex-1 bg-border" />}
			</div>
			<div className={cn('min-w-0 flex-1', !isLast && 'pb-4')}>
				<div className="flex items-center gap-2">
					<TypeIcon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
					<span className="min-w-0 flex-1 truncate text-sm">{step.title}</span>
					<span className="shrink-0 rounded-sm bg-muted px-1.5 py-0.5 font-mono text-[11px] text-muted-foreground">
						{step.type}
					</span>
					<Button
						aria-label={inspectLabel}
						onClick={() => onInspect({ title: step.title, type: step.type })}
						size="icon-xs"
						variant="ghost"
					>
						<InfoIcon aria-hidden="true" />
					</Button>
				</div>
				{step.error && (
					<pre className="mt-1.5 whitespace-pre-wrap break-words rounded-md bg-destructive/8 p-2 font-mono text-xs text-destructive dark:bg-destructive/16">
						{step.error}
					</pre>
				)}
			</div>
		</li>
	);
}

/**
 * The "Activity" tab: the live step-by-step run when one is in flight, the
 * outcome of the last one when it isn't, or an explanation of why there's
 * nothing at all -- distinct for a Discovered (never-installed) arrow versus
 * one that's simply never been run.
 */
export function StepsTimeline({ activeRun, lastReturn, userInstalled }: StepsTimelineProps): JSX.Element {
	const { t } = useTranslation();
	const [inspecting, setInspecting] = useState<StepYamlStep | null>(null);

	const title = activeRun
		? methodLabel(t, activeRun.method)
		: lastReturn
			? methodLabel(t, lastReturn.method)
			: t('arrow.activity.title');

	const OutcomeIcon = lastReturn ? OUTCOME_ICONS[lastReturn.outcome] : undefined;

	return (
		<Frame>
			<FrameHeader>
				<FrameTitle>{title}</FrameTitle>
				{!activeRun && lastReturn && OutcomeIcon && (
					<FrameDescription className="flex items-center gap-1.5">
						<OutcomeIcon aria-hidden="true" className="size-3.5 shrink-0" />
						<span>{outcomeLabel(t, lastReturn.outcome)}</span>
					</FrameDescription>
				)}
			</FrameHeader>
			<FramePanel>
				{activeRun ? (
					<ol className="flex flex-col">
						{activeRun.steps.map((step, index) => (
							<StepRow
								inspectLabel={t('arrow.step.inspect')}
								isLast={index === activeRun.steps.length - 1}
								key={step.index}
								onInspect={setInspecting}
								step={step}
							/>
						))}
					</ol>
				) : lastReturn && OutcomeIcon ? (
					<div className="flex items-center gap-2 text-sm">
						<OutcomeIcon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
						<span>{outcomeLabel(t, lastReturn.outcome)}</span>
					</div>
				) : (
					<p className="py-6 text-center text-sm text-muted-foreground">
						{userInstalled ? t('arrow.activity.empty') : t('arrow.activity.emptyNotInstalled')}
					</p>
				)}
			</FramePanel>
			<StepYamlModal
				// Fully controlled from here (no DialogTrigger of its own) -- Base UI
				// only ever calls this to report a close, never to request an open,
				// so clearing the selection unconditionally is correct, not a shortcut.
				onOpenChange={() => setInspecting(null)}
				open={inspecting !== null}
				step={inspecting}
			/>
		</Frame>
	);
}
