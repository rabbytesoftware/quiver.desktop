import type { JSX } from 'react';

import type { Icon as PhosphorIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { TabsTab } from '@/components/ui/tabs';
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
	/** Matches `PrimaryNav`'s derived value; Base UI raises the indicator over it. */
	value: string;
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

/** crowbar's `sidebar-tab-bar.tsx`, verbatim. */
const TAB = 'flex flex-1 items-center justify-center gap-1';

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
 * A `TabsTab` rendered THROUGH the router's `Link`, so the segment stays a real
 * anchor — middle-click, copy-link and keyboard activation keep working, and
 * navigation stays the router's. Base UI supplies only the active marking and
 * the sliding indicator.
 *
 * Weight follows state — Phosphor `fill` when active, `regular` otherwise — and
 * the glyph takes crowbar's `size={14}` prop rather than a class. A number is
 * safe there; only a `var()` would not be, since a presentation attribute
 * carrying one is substituted by Chromium and dropped by engines that are not,
 * and this app ships in three webviews.
 *
 * The tooltip is Quiver's, not crowbar's, and it stays because this rail
 * collapses to 160px where crowbar's does not: below 280px every segment is
 * icon-only and the tooltip is the only affordance a pointer has. It goes
 * INSIDE the tab — wrapping one makes Base UI's render composition overwrite
 * `data-slot="tabs-tab"` with the tooltip's own.
 */
export function NavSegment({ to, value, active, icon: Icon, label }: NavSegmentProps): JSX.Element {
	const glyph = <Icon size={14} weight={active ? 'fill' : 'regular'} />;

	return (
		<TabsTab value={value} className={TAB} render={<Link to={to} onClick={blockReselect} aria-label={label} />}>
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
		</TabsTab>
	);
}
