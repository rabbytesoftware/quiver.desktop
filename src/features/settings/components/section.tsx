import { useRef, type MouseEvent, type ReactNode } from 'react';

// Phosphor's `*Icon` suffix, matching GearIcon/HouseIcon in the sidebar.
import { ArrowCounterClockwiseIcon } from '@phosphor-icons/react';

import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

export function Section({ title, children }: { title: string; children: ReactNode }) {
	return (
		<section className="px-1 py-0.5 first:[&>[data-slot=section-header]]:hidden">
			<div data-slot="section-header" className="mb-2 px-1 pt-4 pb-1.5">
				<h3 className="text-[13px] font-medium text-foreground">{title}</h3>
			</div>
			<div className="flex flex-col gap-2">{children}</div>
		</section>
	);
}

export function Notice({ tone = 'default', children }: { tone?: 'default' | 'error'; children: ReactNode }) {
	return (
		<p
			role={tone === 'error' ? 'alert' : undefined}
			className={cn(
				'mx-1 mb-3 rounded-md border px-2.5 py-2 text-xs leading-relaxed',
				tone === 'error'
					? 'border-destructive/40 bg-destructive/10 text-destructive'
					: 'border-border bg-muted/45 text-muted-foreground'
			)}
		>
			{children}
		</p>
	);
}

const PASSTHROUGH = "button, input, select, textarea, a, label, [role='switch'], [role='button']";

// Mouse-only convenience: clicking the row's dead space activates its
// control. Clicks that already landed on something interactive are left
// alone — those handle themselves, and forwarding would double-fire.
// There is deliberately no keyboard counterpart and no tabIndex here: the
// control is natively tabbable, so a row-level stop would double every
// tab stop in Settings.
function activate(event: MouseEvent<HTMLDivElement>) {
	// React replays events through the COMPONENT tree, not the DOM tree, so a
	// click inside a portalled popup — a Select's option list is rendered into
	// `document.body` — still arrives here. Those clicks are not "the row's
	// dead space"; forwarding them refocuses the trigger and re-clicks it,
	// which is how selecting an option left the dropdown open. Anything not
	// physically inside this row belongs to whatever portalled it.
	if (!event.currentTarget.contains(event.target as Node)) return;
	if ((event.target as HTMLElement).closest(PASSTHROUGH)) return;
	// A drag-select that ends with `mouseup` inside the row (most usefully,
	// inside the description) still fires `click`. Without this, releasing
	// the mouse after selecting text would immediately clobber that
	// selection by focusing/activating the control instead.
	if (window.getSelection()?.toString()) return;
	const control = event.currentTarget.querySelector<HTMLElement>(`[data-slot=setting-control] ${PASSTHROUGH}`);
	if (!control) return;
	control.focus();
	if (control instanceof HTMLInputElement && control.type === 'number') control.select();
	else if (!(control instanceof HTMLSelectElement)) control.click();
}

export function SettingRow({
	label,
	description,
	children,
	onReset,
	canReset = Boolean(onReset),
}: {
	label: string;
	description?: string;
	children?: ReactNode;
	onReset?: () => void | Promise<void>;
	canReset?: boolean;
}) {
	const { t } = useTranslation();
	const rowRef = useRef<HTMLDivElement>(null);

	// `canReset` flips false the instant a reset succeeds, and `disabled`
	// (plus `invisible`, below) drops the button out of the focus chain —
	// either alone would otherwise dump a keyboard user's focus onto
	// `<body>`. Moving focus to the control the reset just restored keeps it
	// somewhere meaningful instead.
	//
	// `onReset` may be async (e.g. a patch to the daemon), and its control
	// may be keyed on the value the reset changes — so the node present
	// before `await` can be unmounted and replaced by the time it resolves.
	// The query is therefore repeated *after* the await, not reused from
	// before it, so focus always lands on the control that is actually on
	// screen once the reset has taken effect.
	async function handleReset() {
		await onReset?.();
		rowRef.current?.querySelector<HTMLElement>(`[data-slot=setting-control] ${PASSTHROUGH}`)?.focus();
	}

	return (
		<div
			ref={rowRef}
			role="presentation"
			onClick={activate}
			className="flex items-center justify-between gap-3 rounded-lg px-1 py-2 transition-colors hover:bg-muted/50 focus-within:bg-muted/50"
		>
			<div className="min-w-0 flex-1">
				<div className="flex select-none items-center gap-1.5">
					<span className="text-[13px] leading-tight text-foreground">{label}</span>
					{onReset && (
						<button
							type="button"
							onClick={handleReset}
							disabled={!canReset}
							aria-label={t('settings.row.reset', { setting: label })}
							className={cn(
								'grid size-5 place-items-center rounded-md text-muted-foreground',
								'hover:bg-accent hover:text-foreground',
								!canReset && 'pointer-events-none invisible'
							)}
						>
							<ArrowCounterClockwiseIcon size={12} />
						</button>
					)}
				</div>
				{description && (
					<div className="mt-0.5 select-text text-xs leading-relaxed text-muted-foreground">
						{description}
					</div>
				)}
			</div>
			{children && (
				<div data-slot="setting-control" className="flex shrink-0 items-center gap-2">
					{children}
				</div>
			)}
		</div>
	);
}
