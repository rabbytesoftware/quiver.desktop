import type { JSX } from 'react';

/** Matches the cell, not just the tile: art, then the two caption lines. */
export function CardSkeleton({ count }: { count: number }): JSX.Element {
	return (
		<>
			{Array.from({ length: count }, (_unused, index) => (
				<div aria-hidden="true" data-slot="card-skeleton" key={index}>
					<div className="aspect-[2/1] animate-pulse rounded-lg bg-sidebar-element-idle" />
					<div className="px-0.5 pt-[7px]">
						<div className="h-2 w-[58%] animate-pulse rounded-[3px] bg-sidebar-element-idle" />
						<div className="mt-1.5 h-[7px] w-[86%] animate-pulse rounded-[3px] bg-sidebar-element-idle opacity-65" />
					</div>
				</div>
			))}
		</>
	);
}
