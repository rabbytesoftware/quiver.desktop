import type { MouseEvent, ReactNode } from 'react';

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

export function Notice({ children }: { children: ReactNode }) {
	return (
		<p className="mx-1 mb-3 rounded-md border border-border bg-muted/45 px-2.5 py-2 text-xs leading-relaxed text-muted-foreground">
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
	if ((event.target as HTMLElement).closest(PASSTHROUGH)) return;
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
	onReset?: () => void;
	canReset?: boolean;
}) {
	const { t } = useTranslation();

	return (
		<div
			role="presentation"
			onClick={activate}
			className="flex select-none items-center justify-between gap-3 rounded-lg px-1 py-2 transition-colors hover:bg-muted/50 focus-within:bg-muted/50"
		>
			<div className="min-w-0 flex-1">
				<div className="flex items-center gap-1.5">
					<span className="text-[13px] leading-tight text-foreground">{label}</span>
					{onReset && (
						<button
							type="button"
							onClick={onReset}
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
					<div className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{description}</div>
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
