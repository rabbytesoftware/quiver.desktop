import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

import type { LucideIcon } from 'lucide-react';

/**
 * The three destinations the nav can point at. A `string` here would compile and
 * then 404 at runtime: `<Link to>` only type-checks against the generated route
 * tree when it is given a literal, and widening the prop is what throws that
 * check away.
 */
export type NavDestination = '/' | '/remote' | '/settings';

export interface NavSegmentProps {
	to: NavDestination;
	/**
	 * Whether the segment lights up only on its own route rather than on
	 * everything beneath it. Off for the leaves, and required for `/` — see
	 * `primary-nav.tsx`.
	 */
	exact?: boolean;
	icon: LucideIcon;
	/** Already translated. Doubles as the accessible name and the tooltip text. */
	label: string;
}

/**
 * The full width of the rail is divided between the three segments, always.
 *
 * The active one is capped at 54% — the proportion design.pen gives it, 112 of
 * 208. Uncapped it eats the rail at SIDEBAR_MAX; sized to its label alone it is
 * too tight at SIDEBAR_MIN. The other two split what is left, so there is never
 * dead space, and with nothing active all three fall through to the collapsed
 * rule and share equally. That is why there is no third selector for "nothing
 * active" here: three inactive links already are it.
 *
 * `not-data-[status=active]:` rather than a later rule that overrides the active
 * fill. An override still paints for the frame before it wins, so the hover
 * flickers as the cursor crosses the active segment. Same reasoning as the
 * history buttons' `not-disabled:`.
 *
 * No gap between segments. design.pen's `gap: 4` existed only to make
 * `112 + 4 + 44 + 4 + 44` come out to 208; the rail is 246 now, the active
 * segment absorbs the difference, and nothing else in the rail is separated.
 */
const SEGMENT = [
	'flex h-(--row) items-center overflow-hidden text-[13px]',
	'data-[status=active]:max-w-[54%] data-[status=active]:flex-[1_1_auto]',
	'data-[status=active]:gap-(--inset) data-[status=active]:px-(--inset)',
	'data-[status=active]:bg-sidebar-primary data-[status=active]:text-sidebar-primary-foreground',
	'not-data-[status=active]:min-w-(--row) not-data-[status=active]:flex-1',
	'not-data-[status=active]:hover:bg-sidebar-accent',
].join(' ');

/** `--icon`, the content tier — one step above the history chevrons' `--icon-chrome`. */
const ICON = 'size-(--icon) shrink-0';

/**
 * One destination in the rail's primary nav: an icon that grows a label when the
 * router says this is where we are.
 *
 * `isActive` comes out of `<Link>` itself rather than from a second
 * `useMatchRoute` call. The router writing `data-status` is the entire selection
 * model (spec §5.1) — asking it the same question a second time, through a
 * different API with its own spelling of "exact", is how the tooltip ends up
 * disagreeing with the highlight about which segment is open.
 *
 * That is also why the tooltip's trigger is a span inside the link rather than
 * the link itself: `isActive` is only in scope below `<Link>`. The cost is that
 * the tooltip opens on hover but not on keyboard focus, which is why `aria-label`
 * is unconditional — the accessible name is what serves assistive tech here, and
 * the tooltip is the pointer affordance on top of it.
 */
export function NavSegment({ to, exact = false, icon: Icon, label }: NavSegmentProps): JSX.Element {
	return (
		<Link to={to} activeOptions={{ exact }} aria-label={label} className={SEGMENT}>
			{({ isActive }) =>
				isActive ? (
					<>
						<Icon className={ICON} />
						<span className="min-w-0 truncate">{label}</span>
					</>
				) : (
					<Tooltip>
						{/* Fills the segment so the whole of it is hoverable, not just
						 * the 20px the icon occupies once flex has handed this segment
						 * a share of the rail wider than a square. */}
						<TooltipTrigger render={<span className="flex size-full items-center justify-center" />}>
							<Icon className={ICON} />
						</TooltipTrigger>
						{/* Below, not above: the rail's top bar is directly above the
						 * nav, so a tooltip on the default side covers the history
						 * buttons. Below is clear on either edge of the window, which
						 * `side="inline-end"` would not be for a right-docked rail. */}
						<TooltipContent side="bottom">{label}</TooltipContent>
					</Tooltip>
				)
			}
		</Link>
	);
}
