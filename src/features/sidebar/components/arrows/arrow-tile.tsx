import type { CSSProperties, JSX, KeyboardEvent, MouseEvent } from 'react';

import { Link } from '@tanstack/react-router';

import { badgeVariants } from '@/components/ui/badge';
import { FlickerSpinner } from '@/components/ui/flicker-spinner';

import type { ArrowStatus } from '@/features/arrow-details/lib/status';
import { STATUS_BADGE_VARIANT, STATUS_ICONS } from '@/features/arrow-details/lib/status';
import { cn } from '@/lib/cn';
import { cssUrl } from '@/lib/css';
import { useTranslation } from '@/lib/i18n';
import { ownerOf } from '@/lib/namespace';

import { ArrowIcon } from './arrow-icon';

import '@/features/search/styles/card.css';

/**
 * The cell is the link, same reasoning as ArrowCard (arrow-card.tsx): the
 * caption is part of the same hit target the banner's hover-lift covers.
 */
const CELL = [
	'group relative block min-w-0 cursor-pointer',
	'hover:z-[2] focus-visible:z-[2]',
	'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground',
].join(' ');

const CARD = 'relative block aspect-[2/1] min-w-0 [--reveal:30px]';

const INFO = [
	'absolute inset-x-0 bottom-0 flex items-center gap-1.5 px-0.5',
	'h-[var(--reveal)] min-w-0 text-foreground',
	'opacity-0 transition-opacity duration-[90ms]',
	'group-hover:opacity-100 group-hover:duration-[40ms]',
	'group-focus-visible:opacity-100 group-focus-visible:duration-[40ms]',
].join(' ');

/**
 * Only `problem` (a detached arrow) has a real recovery action -- a plain
 * Stop. Core has no cancel/abort capability for `busy` states (installing,
 * updating, ...), confirmed against docs/arrow-details-spec.md §8, so those
 * badges are informational only even though they look similar.
 */
const RESOLVABLE_ICON_KIND = 'problem';

export interface ArrowTileProps {
	/** Where the tile navigates on click -- an arrow tile links to its detail page, a collection tile to the collection's own page. Both routes share the same `_splat` param shape. */
	to: '/arrow/$' | '/collection/$';
	namespace: string;
	title: string;
	subtitle: string;
	icon: string | null;
	banner: string | null;
	/** The hover-reveal strip's mono caption -- a version string for an arrow, an item count for a collection. */
	metaText: string;
	/** Omit (or pass `null`) for the steady, unremarkable case -- no badge renders, matching how Search/Collection tiles already stay quiet by default. */
	status?: ArrowStatus | null;
	/** Only called when `status.iconKind === 'problem'`; ignored otherwise. */
	onResolve?: () => void;
}

function bannerStyle(banner: string | null): CSSProperties | undefined {
	return banner ? { backgroundImage: cssUrl(banner) } : undefined;
}

export function ArrowTile({
	to,
	namespace,
	title,
	subtitle,
	icon,
	banner,
	metaText,
	status,
	onResolve,
}: ArrowTileProps): JSX.Element {
	const { t } = useTranslation();

	const StatusIcon = status ? STATUS_ICONS[status.iconKind] : null;
	const canResolve = Boolean(status && status.iconKind === RESOLVABLE_ICON_KIND && onResolve);

	function resolve(event: MouseEvent | KeyboardEvent): void {
		if (!canResolve) return;
		event.preventDefault();
		event.stopPropagation();
		onResolve?.();
	}

	function handleBadgeKeyDown(event: KeyboardEvent): void {
		if (event.key !== 'Enter' && event.key !== ' ') return;
		resolve(event);
	}

	return (
		<Link className={CELL} data-slot="arrow-card" params={{ _splat: namespace }} to={to}>
			<span className={CARD}>
				<span
					aria-hidden="true"
					className="absolute inset-0 overflow-hidden rounded-lg bg-muted bg-cover bg-center"
					data-slot="card-banner"
					style={bannerStyle(banner)}
				>
					{!banner && (
						<span data-slot="card-drawn">
							<span data-slot="drawn-ghost">{title.slice(0, 1).toUpperCase()}</span>
							{icon && <span data-slot="drawn-mark" style={{ backgroundImage: cssUrl(icon) }} />}
							<span data-slot="drawn-type">
								<span data-slot="drawn-name">{title}</span>
								<span data-slot="drawn-owner">{ownerOf(namespace)}</span>
							</span>
						</span>
					)}
				</span>

				<span className={INFO} data-slot="card-info">
					<span className="inline-flex flex-none" style={{ '--icon': '20px' } as CSSProperties}>
						<ArrowIcon icon={icon} name={title} namespace={namespace} />
					</span>
					<span className="min-w-0 flex-1 truncate font-mono text-[9.5px]/[12px] tracking-[-0.2px] opacity-90">
						{metaText}
					</span>
				</span>
			</span>

			<span className="block min-w-0 px-0.5 pt-[7px]">
				<span className="flex min-w-0 items-center gap-1.5">
					<span className="min-w-0 flex-1 truncate text-[12.5px]/[15px] font-medium tracking-[-0.1px]">
						{title}
					</span>
					{status && StatusIcon && (
						// A plain element on badgeVariants' own classes, not the <Badge>
						// component -- Base UI's useRender special-cases role="button"
						// and expects a real <button> in that case (a real <button> here
						// would nest inside this tile's own <Link>, which is invalid);
						// bypassing useRender keeps the interaction a plain, working click.
						<span
							aria-label={canResolve ? t(status.labelKey) : undefined}
							className={cn(
								badgeVariants({ size: 'sm', variant: STATUS_BADGE_VARIANT[status.iconKind] }),
								'shrink-0 gap-1',
								canResolve && 'cursor-pointer hover:brightness-90'
							)}
							data-slot="badge"
							onClick={resolve}
							onKeyDown={canResolve ? handleBadgeKeyDown : undefined}
							role={canResolve ? 'button' : undefined}
							tabIndex={canResolve ? 0 : undefined}
						>
							{status.iconKind === 'busy' ? (
								<FlickerSpinner aria-hidden="true" className="size-2.5" />
							) : (
								<StatusIcon aria-hidden="true" className="size-2.5" />
							)}
							{t(status.labelKey)}
						</span>
					)}
				</span>
				<span className="mt-px block truncate text-[11px]/[14px] text-muted-foreground">{subtitle}</span>
			</span>
		</Link>
	);
}
