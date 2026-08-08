import type { JSX } from 'react';

import type { Icon as PhosphorIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

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
	/**
	 * Whether the segment lights up only on its own route rather than on
	 * everything beneath it. Off for the leaves, and required for `/` — see
	 * `primary-nav.tsx`.
	 */
	exact?: boolean;
	/**
	 * Phosphor's own component type, aliased because the destructure below binds
	 * this prop to `Icon` in order to render it, and an unaliased import would
	 * be shadowed by it.
	 */
	icon: PhosphorIcon;
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
 *
 * 610 and -0.1px are design.pen's own numbers, written literally because no
 * token carries them. The weight lands on the active segment alone, which is
 * the only one that renders text — a collapsed segment inherits it and has
 * nothing to apply it to. Dropping it does not break a layout or a test; it
 * just leaves the one label on screen reading a shade lighter than the design,
 * against a fill that is already inverted and unforgiving about weight.
 */
const SEGMENT = [
	'flex h-(--row) items-center overflow-hidden text-[13px] tracking-[-0.1px]',
	'data-[status=active]:max-w-[54%] data-[status=active]:flex-[1_1_auto]',
	'data-[status=active]:gap-(--inset) data-[status=active]:px-(--inset)',
	'data-[status=active]:font-[610] data-[status=active]:bg-sidebar-primary',
	'data-[status=active]:text-sidebar-primary-foreground',
	'not-data-[status=active]:min-w-(--row) not-data-[status=active]:flex-1',
	'not-data-[status=active]:hover:bg-sidebar-accent',
].join(' ');

/**
 * `--icon-nav` (14), the smallest of the three tiers — NOT `--icon` (20), which
 * belongs to the arrow rows. A nav segment is mostly icon: at 20 the glyph fills
 * a collapsed segment edge to edge and the three of them read as toolbar buttons
 * rather than as one segmented control. The design draws these at 14 and the
 * list at 20 for exactly that reason.
 *
 * A class and not Phosphor's `size` prop, which is not a substitute for it:
 * `size` is written to the svg's `width`/`height` ATTRIBUTES, and `var()` is
 * not substituted in an attribute, so `size="var(--icon-nav)"` would be
 * discarded and leave the icon at the context default of 1em — 13px, tracking
 * the font instead of the token. The class wins over the attribute regardless:
 * presentation attributes lose to every author rule.
 */
const ICON = 'size-(--icon-nav) shrink-0';

/**
 * One weight for all three segments, and `bold` rather than Phosphor's default
 * `regular` because of the size above. Phosphor draws regular at 16 units of a
 * 256 grid and bold at 24, so at 14px they come out at 0.88px and 1.31px of
 * stroke; design.pen draws these glyphs at 2.1 of a 24 grid, which is 1.23px.
 * Bold is the one that matches. Regular is the tempting default and reads
 * visibly wispier than the design at this size, most of all on the inverted
 * fill the active segment paints behind it.
 *
 * Stated rather than left to the default so an `IconContext` mounted anywhere
 * above the rail cannot restyle it from a distance.
 */
const WEIGHT = 'bold';

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
		<Link to={to} activeOptions={{ exact }} onClick={blockReselect} aria-label={label} className={SEGMENT}>
			{({ isActive }) =>
				isActive ? (
					<>
						<Icon className={ICON} weight={WEIGHT} />
						<span className="min-w-0 truncate">{label}</span>
					</>
				) : (
					<Tooltip>
						{/* Fills the segment so the whole of it is hoverable, not just
						 * the 20px the icon occupies once flex has handed this segment
						 * a share of the rail wider than a square. */}
						<TooltipTrigger render={<span className="flex size-full items-center justify-center" />}>
							<Icon className={ICON} weight={WEIGHT} />
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
