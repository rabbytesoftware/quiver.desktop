import type { JSX } from 'react';

import type { Icon as PhosphorIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

import { cn } from '@/lib/cn';

import { blockReselect } from '../reselect';

/**
 * The three destinations the nav can point at. A `string` here would compile and
 * then 404 at runtime: `<Link to>` only type-checks against the generated route
 * tree when it is given a literal, and widening the prop is what throws that
 * check away.
 */
export type NavDestination = '/' | '/remote' | '/settings';

export interface NavSegmentProps {
	to: NavDestination;
	/** True when this is the destination the router is currently on. */
	active: boolean;
	/**
	 * Phosphor's own component type, aliased because the destructure below binds
	 * this prop to `Icon` in order to render it, and an unaliased import would be
	 * shadowed by it.
	 */
	icon: PhosphorIcon;
	/** Already translated. Doubles as the accessible name and the tooltip text. */
	label: string;
}

/**
 * crowbar's `sidebar-tab-bar.tsx` classes, verbatim, plus the two this rail
 * needs: `h-9` — which Base UI's own tab supplied before the segments stopped
 * being tabs — and `relative z-10`, which puts the segment above the travelling
 * indicator so the fill passes underneath its icon and label.
 */
const TAB = [
	// crowbar's TabsTab, verbatim — including `sm:h-8`, which is what makes the
	// segment 32px at any window this app runs at. Pinning it to h-9 made the
	// whole bar 4px taller than the control it was copied from.
	'relative flex h-9 shrink-0 grow cursor-pointer items-center justify-center gap-1.5',
	'whitespace-nowrap rounded-md border border-transparent px-[calc(--spacing(2.5)-1px)]',
	'font-medium text-base outline-none transition-[color,background-color,box-shadow]',
	'focus-visible:ring-2 focus-visible:ring-ring sm:h-8 sm:text-sm',
	// crowbar's per-call className from sidebar-tab-bar.tsx
	'flex flex-1 items-center justify-center gap-1',
	// Ours: z-10 keeps the segment above the travelling indicator, and the
	// active colour inverts where crowbar's lifts. The delay is the arriving
	// side only — see ROW_ACTIVE.
	'z-10 data-[status=active]:text-background data-[status=active]:[transition:color_0ms_200ms]',
].join(' ');

/**
 * The label's visibility ladder, also crowbar's, thresholds included: under
 * 280px nobody gets a label, 280–420 only the active tab, 420 and up all of
 * them. Measured against the CONTAINER — this rail is dragged between 160 and
 * 320px and the window's width says nothing about it.
 *
 * At Quiver's 246px default the container is 230px, so the bar is icons only
 * and the labels arrive as the rail is widened. That falls out of the control's
 * own thresholds rather than being a number picked for this app.
 */
function labelClass(active: boolean): string {
	return cn('hidden', active && '@[280px]:inline', '@[420px]:inline');
}

/**
 * One destination in the rail's primary nav.
 *
 * A plain router `<Link>`. It was a Base UI `TabsTab` rendered through one,
 * which gave the segment `role="tab"` and its own sliding indicator — and both
 * had to go once the whole rail took a single travelling indicator, because
 * `Tabs.Indicator` can only measure inside its own list. What is left is what
 * these always were semantically: three destinations, competing with every
 * arrow below them. Nothing here is a tabpanel.
 *
 * `activeOptions={{ exact }}` for Home is load-bearing. TanStack matches by
 * prefix and `/` is a prefix of every route in the app, so without it Home is
 * lit on top of Remote, on top of Settings, on top of every open arrow.
 *
 * Weight follows state — Phosphor `fill` when active, `regular` otherwise — at
 * crowbar's `size={14}`. A literal is safe in that prop; only a `var()` would
 * not be, since it writes to the svg's width/height attributes and an engine
 * that does not substitute custom properties there falls back to 1em.
 *
 * The tooltip is Quiver's, not crowbar's, and it stays because this rail
 * collapses to 160px where crowbar's does not: below 280px every segment is
 * icon-only and the tooltip is the only affordance a pointer has.
 */
export function NavSegment({ to, active, icon: Icon, label }: NavSegmentProps): JSX.Element {
	const glyph = <Icon size={14} weight={active ? 'fill' : 'regular'} />;

	return (
		<Link to={to} activeOptions={{ exact: to === '/' }} onClick={blockReselect} aria-label={label} className={TAB}>
			{active ? (
				glyph
			) : (
				<Tooltip>
					<TooltipTrigger render={<span className="flex items-center justify-center" />}>
						{glyph}
					</TooltipTrigger>
					{/* Below, not above: the rail's top bar sits directly over the nav,
					 * so a tooltip on the default side covers the history buttons.
					 * Below is clear on either edge of the window, which
					 * `side="inline-end"` would not be for a right-docked rail. */}
					<TooltipContent side="bottom">{label}</TooltipContent>
				</Tooltip>
			)}
			<span className={labelClass(active)}>{label}</span>
		</Link>
	);
}
