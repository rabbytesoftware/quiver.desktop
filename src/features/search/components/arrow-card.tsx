import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import type { SearchEntry } from '@/domain/search';
import { cn } from '@/lib/cn';

import './card.css';

/**
 * The lift distance and the strip height at once, so the banner can never
 * uncover more or less than the strip fills. Task 7's grid padding is derived
 * from this -- see spec 8.2 and 9.2.
 */
const CARD = [
	'group relative block aspect-[2/1] min-w-0 cursor-pointer',
	'[--reveal:30px]',
	// No clip and no ground of its own: on hover the card paints over whatever
	// is above it and grows out of its cell rather than being masked by it.
	'hover:z-[2] focus-visible:z-[2]',
	'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground',
].join(' ');

const INFO = [
	'absolute inset-x-0 bottom-0 flex items-center gap-1.5 px-0.5',
	'h-[var(--reveal)] min-w-0 text-foreground',
	// Gone (90ms) before the rebound starts (~117ms), or the kick flashes half
	// a name back into view. Spec 8.6.
	'opacity-0 transition-opacity duration-[90ms]',
	'group-hover:opacity-100 group-hover:duration-[40ms]',
	'group-focus-visible:opacity-100 group-focus-visible:duration-[40ms]',
].join(' ');

export function ArrowCard({ entry }: { entry: SearchEntry }): JSX.Element {
	return (
		<Link
			className={CARD}
			data-slot="arrow-card"
			{...(entry.provenance ? { 'data-provenance': entry.provenance } : {})}
			params={{ _splat: entry.namespace }}
			to="/arrow/$"
		>
			<span
				aria-hidden="true"
				className="absolute inset-0 rounded-lg bg-cover bg-center will-change-transform"
				data-slot="card-banner"
				style={entry.banner ? { backgroundImage: `url(${entry.banner})` } : undefined}
			/>
			<span className={INFO} data-slot="card-info">
				<span
					aria-hidden="true"
					className="size-5 flex-none overflow-hidden rounded-[5px] bg-cover bg-center"
					style={entry.icon ? { backgroundImage: `url(${entry.icon})` } : undefined}
				/>
				<span className="flex min-w-0 flex-col justify-center">
					<span className="truncate text-[12.5px]/[15px] font-medium tracking-[-0.1px]">{entry.name}</span>
					{/* rtl truncates the namespace at the front, keeping the arrow
					    name visible rather than the host. Spec 8.8. */}
					<span
						className={cn(
							'truncate font-mono text-[8px]/[11px] tracking-[-0.1px] opacity-55',
							'[direction:rtl] text-left'
						)}
					>
						{entry.namespace}
					</span>
				</span>
			</span>
		</Link>
	);
}
