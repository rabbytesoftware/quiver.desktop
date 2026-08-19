import type { JSX } from 'react';

export function CardSkeleton({ count }: { count: number }): JSX.Element {
	return (
		<>
			{Array.from({ length: count }, (_unused, index) => (
				<div
					aria-hidden="true"
					className="aspect-[2/1] animate-pulse rounded-lg bg-sidebar-element-idle"
					data-slot="card-skeleton"
					key={index}
				/>
			))}
		</>
	);
}
