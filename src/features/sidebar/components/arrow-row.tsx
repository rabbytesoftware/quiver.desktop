import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import type { ArrowEntry } from '@/domain/arrow';
import { cn } from '@/lib/cn';

import { splitNamespace } from '../namespace';
import { blockReselect } from '../reselect';
import { ROW_ACTIVE, ROW_BASE, ROW_INACTIVE, ROW_SUBLABEL } from '../row-base';
import { ArrowIcon } from './arrow-icon';

/**
 * `group` so the subtitle can key off the row's own `data-status` — the router
 * writes it here, and nothing below can see it otherwise.
 */
const ROW = cn(ROW_BASE, ROW_INACTIVE, ROW_ACTIVE, 'group');

/**
 * The subtitle is `hidden` until the ROUTER says otherwise, not switched by a
 * React branch. The rail sits above the `<Outlet/>`, so component state here
 * would be a second copy of the selection that spec §5.1 exists to avoid, and
 * it would have to re-render on every navigation to stay honest.
 */
const SUBTITLE = cn(ROW_SUBLABEL, 'hidden group-data-[status=active]:flex');

interface ArrowRowProps {
	arrow: ArrowEntry;
}

/**
 * One installed arrow, as a rail row.
 *
 * The row's height does not change when the subtitle appears. `ROW_BASE`'s
 * `h-9` is a fixed 36px and the two leadings — 16px for the name, 13px for the
 * subtitle — are sized to fit inside it. A row that grew to fit its second line
 * would shove every row beneath it down at the moment of selection, which is
 * exactly when the eye is following something else.
 */
export function ArrowRow({ arrow }: ArrowRowProps): JSX.Element {
	const { head, tail } = splitNamespace(arrow.namespace);

	return (
		<Link to="/arrow/$" params={{ _splat: arrow.namespace }} onClick={blockReselect} className={ROW}>
			<ArrowIcon namespace={arrow.namespace} name={arrow.name} icon={arrow.icon} />
			{/* `min-w-0` on the column, or `truncate` below it has nothing to
			 * shrink against and the name overflows the rail instead of ending in
			 * an ellipsis. */}
			<span className="flex min-w-0 flex-1 flex-col justify-center">
				{/* Inter, not the mono. An arrow's display name is a name; mono is
				 * reserved for the identifiers on the line below. */}
				<span data-slot="arrow-name" className="truncate text-[13px]/[16px]">
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
