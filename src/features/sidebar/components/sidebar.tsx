import type { JSX } from 'react';

import { useShellStore } from '@/features/shell';
import { cn } from '@/lib/cn';

import { ArrowList } from './arrow-list';
import { PrimaryNav } from './primary-nav';
import { RailTopBar } from './rail-top-bar';
import { ResizeHandle } from './resize-handle';

export interface SidebarProps {
	/**
	 * The rail's grid column, from `AppShell`.
	 *
	 * Derived there and not here on purpose: the template and the placements
	 * have to reverse together, and a second copy of the side→column mapping
	 * drifts into the rail being auto-placed in the `1fr` track — a 246px
	 * column of content beside a full-width rail, with no setting on screen
	 * that looks wrong.
	 */
	className?: string;
}

/**
 * The rail: one column of the window, top to bottom (spec §1.1).
 *
 * `relative` IS LOAD-BEARING. `ResizeHandle` is `absolute inset-y-0`, so with
 * no positioned ancestor here it resolves against whatever further up the tree
 * happens to be positioned — or the viewport — and the four-pixel drag strip
 * lands somewhere across the window instead of on the rail's edge. Nothing
 * about the rail looks wrong; it just cannot be resized with a pointer.
 *
 * The divider rides the rail rather than the content column (spec §1.3). The
 * content occupies row 2 only, so a border over there stops short of the top
 * and leaves the chrome row undivided.
 */
export function Sidebar({ className }: SidebarProps): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);

	return (
		<div
			data-slot="sidebar"
			className={cn(
				// `min-h-0` because a grid item's default `min-height: auto`
				// refuses to shrink below its content: without it a long library
				// grows the rail past its track, `flex-1` inside resolves against
				// the grown height, and the list never scrolls — it is simply
				// clipped by the shell, with the rows below the fold unreachable.
				'relative row-span-2 flex min-h-0 flex-col',
				'bg-sidebar text-sidebar-foreground border-sidebar-border',
				side === 'left' ? 'border-r' : 'border-l',
				className
			)}
		>
			<RailTopBar />
			<PrimaryNav />
			{/* The only part that scrolls. Give the scroll to the rail instead and
			    the top bar and the three destinations leave the screen as soon as
			    the library is longer than the window — including the back button,
			    which is then the one control you cannot reach. */}
			<ArrowList />
			<ResizeHandle />
		</div>
	);
}
