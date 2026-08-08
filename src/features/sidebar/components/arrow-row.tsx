import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import type { ArrowEntry } from '@/domain/arrow';

import { splitNamespace } from '../namespace';
import { ArrowIcon } from './arrow-icon';

/**
 * The active treatment is a nav segment's, to the token: an arrow row and
 * Home are the same navigation, so they answer the same way (spec §5.2).
 *
 * Hover EXCLUDES the active row rather than being overridden by a later rule.
 * An override still paints the fill for the frame before it wins, so the active
 * row flickers as the cursor crosses it on the way to another one.
 */
const ROW =
	'group flex h-(--row) items-center gap-(--inset) p-(--inset) ' +
	'data-[status=active]:bg-sidebar-primary data-[status=active]:text-sidebar-primary-foreground ' +
	'not-data-[status=active]:hover:bg-sidebar-accent';

/**
 * The subtitle only ever paints on the selected row, which is the inverted
 * surface. `text-muted-foreground` is tuned against the content column and
 * disappears there; `text-current/70` de-emphasises whatever the row's own
 * foreground currently is, on either surface.
 *
 * `hidden` until the ROUTER says otherwise — not a React branch. The rail sits
 * above the `<Outlet/>`, so component state here would be a second copy of the
 * selection that §5.1 exists to not have, and it would have to re-render on
 * every navigation to stay honest.
 */
const SUBTITLE = 'hidden min-w-0 text-[10px] text-current/70 group-data-[status=active]:flex';

interface ArrowRowProps {
	arrow: ArrowEntry;
}

/**
 * One installed arrow, as a rail row.
 *
 * The row's height does not change when the subtitle appears: 13px/1.25 plus
 * 10px/1.25 is 28.75, inside `--row`'s 34. A row that grew instead would shove
 * every row below it down at the moment of selection.
 */
export function ArrowRow({ arrow }: ArrowRowProps): JSX.Element {
	const { head, tail } = splitNamespace(arrow.namespace);

	return (
		<Link to="/arrow/$" params={{ _splat: arrow.namespace }} className={ROW}>
			<ArrowIcon name={arrow.name} icon={arrow.icon} />
			{/* `min-w-0` on the column, or `truncate` below it has nothing to
			 * shrink against and the name overflows the rail instead of ending
			 * in an ellipsis. */}
			<span className="flex min-w-0 flex-1 flex-col justify-center leading-[1.25]">
				<span data-slot="arrow-name" className="truncate">
					{arrow.name}
				</span>
				{/* Two spans, not one string: the head gives up the middle of the
				 * path and the tail is pinned, so the version — the useful end —
				 * survives every width down to the 160px floor. Truncating the
				 * whole namespace would drop it first. */}
				<span data-slot="arrow-namespace" className={SUBTITLE}>
					<span className="truncate">{head}</span>
					<span className="shrink-0">{tail}</span>
				</span>
			</span>
		</Link>
	);
}
