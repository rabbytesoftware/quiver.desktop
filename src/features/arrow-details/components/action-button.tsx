import { useState, type JSX } from 'react';

import { FlickerSpinner } from '@/components/ui/flicker-spinner';

import type { ArrowVariable } from '@/domain/arrow';
import type { ArrowAction, ArrowActionKind, ArrowActionVariant } from '@/features/arrow-details/lib/actions';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

import {
	DownloadIcon,
	InfoIcon,
	PlayIcon,
	PlusIcon,
	RefreshCwIcon,
	SquareIcon,
	Trash2Icon,
	type LucideIcon,
} from 'lucide-react';

import { StepPreviewModal } from './step-preview-modal';

const ACTION_ICONS: Partial<Record<ArrowActionKind, LucideIcon>> = {
	addToLibrary: PlusIcon,
	install: DownloadIcon,
	reinstall: RefreshCwIcon,
	execute: PlayIcon,
	restart: RefreshCwIcon,
	stop: SquareIcon,
	update: RefreshCwIcon,
	uninstall: Trash2Icon,
};

/**
 * One fused split-button per variant, not a shared radius/border baked into
 * `buttonVariants` -- the container alone owns the border, background, and
 * shadow; each zone inside it is a plain transparent button that only tints
 * on hover/press, matching the hero design's own split-button component.
 * Colors are drawn from the same tokens `buttonVariants` itself uses
 * (`--primary`, `--destructive`, `--input`, `--popover`, `--accent`), just
 * arranged for a shared container instead of one button per variant.
 */
const SPLIT_VARIANTS: Record<ArrowActionVariant, { container: string; zone: string; divider: string }> = {
	default: {
		container:
			'border-primary bg-primary text-primary-foreground shadow-primary/24 shadow-xs not-disabled:inset-shadow-[0_1px_--theme(--color-white/16%)]',
		zone: 'hover:bg-white/14 active:bg-black/8',
		divider: 'bg-white/20',
	},
	destructive: {
		container:
			'border-destructive bg-destructive text-white shadow-destructive/24 shadow-xs not-disabled:inset-shadow-[0_1px_--theme(--color-white/16%)]',
		zone: 'hover:bg-white/14 active:bg-black/8',
		divider: 'bg-white/25',
	},
	'destructive-outline': {
		container: 'border-input bg-popover not-dark:bg-clip-padding text-destructive shadow-xs/5 dark:bg-input/32',
		zone: 'hover:bg-destructive/4',
		divider: 'bg-destructive/32',
	},
	outline: {
		container: 'border-input bg-popover not-dark:bg-clip-padding text-foreground shadow-xs/5 dark:bg-input/32',
		zone: 'hover:bg-accent/50 dark:hover:bg-input/64',
		divider: 'bg-border',
	},
};

interface ActionButtonProps {
	action: ArrowAction;
	/** True while this specific action's mutation call is in flight, before the server-confirmed `active_run` (and thus `action.forceBusy`) catches up via the WebSocket. */
	pending: boolean;
	onInvoke: () => void;
	variables: ArrowVariable[];
	values: Record<string, string>;
	onValueChange: (name: string, value: string) => void;
}

/**
 * A primary hero action fused with its "what this does" info trigger into a
 * single split-button: one rounded, bordered container, a hairline divider,
 * two independently-hoverable transparent zones -- not two separate `Button`s
 * with a gap between them. The info zone is omitted entirely when there's
 * nothing to preview (no steps, e.g. "Add to Library"), matching the design.
 */
export function ActionButton({
	action,
	pending,
	onInvoke,
	variables,
	values,
	onValueChange,
}: ActionButtonProps): JSX.Element {
	const { t } = useTranslation();
	const [infoOpen, setInfoOpen] = useState(false);

	const busy = action.forceBusy || pending;
	const disabled = busy || action.forceDisabled;
	const hasInfo = action.steps.length > 0;
	const Icon = ACTION_ICONS[action.kind];
	const label = busy && action.busyLabelKey ? t(action.busyLabelKey) : t(action.labelKey);
	const split = SPLIT_VARIANTS[action.variant];

	return (
		<div
			className={cn(
				'inline-flex h-9 items-stretch overflow-hidden rounded-md border text-base font-medium sm:h-8 sm:text-sm',
				// Direct-child-of-button only, not `[&_svg]` (any descendant) --
				// FlickerSpinner's own inner svg relies on its `h-full w-full`
				// sizing to stay clipped inside its 8-frame sprite strip, and a
				// deep descendant selector here would force it to a fixed square
				// and break the animation. lucide icons ARE direct button children,
				// so this still reaches them correctly.
				'[&>button>svg]:pointer-events-none [&>button>svg]:size-4.5 [&>button>svg]:shrink-0 sm:[&>button>svg]:size-4',
				split.container,
				disabled && 'opacity-64'
			)}
		>
			{/* Not `Button`'s own `loading` prop: that swaps in the generic Loader2
			    spinner and makes the label text transparent, which is wrong here --
			    the busy label ("Installing…") must stay visible, with FlickerSpinner
			    placed after it rather than overlaid on top. */}
			<button
				className={cn(
					'inline-flex cursor-pointer items-center gap-1.5 whitespace-nowrap px-[calc(--spacing(3)-1px)] outline-none transition-shadow focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:cursor-not-allowed',
					split.zone
				)}
				disabled={disabled}
				onClick={onInvoke}
				type="button"
			>
				{!busy && Icon && <Icon aria-hidden="true" />}
				{label}
				{busy && <FlickerSpinner aria-hidden="true" />}
			</button>
			{hasInfo && (
				<>
					<span aria-hidden="true" className={cn('w-px shrink-0 self-stretch', split.divider)} />
					<button
						aria-label={t('arrow.action.info')}
						className={cn(
							'inline-flex w-9 shrink-0 cursor-pointer items-center justify-center outline-none transition-shadow sm:w-8 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background',
							split.zone
						)}
						onClick={() => setInfoOpen(true)}
						type="button"
					>
						<InfoIcon aria-hidden="true" />
					</button>
				</>
			)}

			{hasInfo && (
				<StepPreviewModal
					onOpenChange={setInfoOpen}
					onValueChange={onValueChange}
					open={infoOpen}
					steps={action.steps}
					title={t(action.labelKey)}
					usesVariables={action.usesVariables}
					values={values}
					variables={variables}
				/>
			)}
		</div>
	);
}
