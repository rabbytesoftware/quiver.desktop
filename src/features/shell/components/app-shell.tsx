import type { CSSProperties, JSX, ReactNode } from 'react';

import { TooltipProvider } from '@/components/ui/tooltip';

import { Sidebar } from '@/features/sidebar';
import { cn } from '@/lib/cn';

import { useShellStore } from '../store';
import { ChromeRow } from './chrome-row';

export interface AppShellProps {
	/** The content column's row 2 — the router's outlet. */
	children: ReactNode;
	/**
	 * Pinned to the bottom of the content column, below the scroll region and
	 * inside the same track — so it never runs past the rail.
	 *
	 * A slot rather than an import, because the only thing that goes here is the
	 * dev-only mock band: the shell would otherwise reach into `@/lib/mock` to
	 * render something a release build never shows, and every consumer of the
	 * shell would carry that dependency.
	 */
	footer?: ReactNode;
}

/**
 * The whole window: two columns, two rows, and the rail spanning both rows of
 * whichever column it is docked to (spec §1).
 *
 * Every one of the three cells is placed explicitly. Auto-placement fills row
 * by row, so the moment the template reverses it would drop the rail into the
 * `1fr` track and the content into the `--rail` one — the rail rendered as the
 * content column, at 246px, with no setting on screen that looks wrong.
 */
export function AppShell({ children, footer }: AppShellProps): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);
	const width = useShellStore((s) => s.sidebarWidth);

	const railColumn = side === 'left' ? 'col-start-1' : 'col-start-2';
	const contentColumn = side === 'left' ? 'col-start-2' : 'col-start-1';

	return (
		// One provider for the whole tree (spec §3.2). Mounted per tooltip
		// instead, every collapsed nav segment would open on its own delay clock
		// and the rail would stop behaving as one control strip.
		<TooltipProvider>
			<div
				// `data-shell` is a contract with `ResizeHandle`, which resolves
				// its write target as `closest('[data-shell], [style*="--rail"]')`
				// — drop the attribute and a drag silently retargets whatever
				// else happens to carry the property.
				data-shell
				// `--rail` is declared HERE and not only on `:root`. The handle
				// writes the live width to this element on every pointermove, and
				// an inline declaration outranks the `:root` rule — so with the
				// property left on the root alone this style would overrule sixty
				// writes a second and the rail would sit still until the store
				// commits on pointer-up, then snap.
				style={{ '--rail': `${width}px` } as CSSProperties}
				className={cn(
					// `h-screen` again: nothing stacks above the grid any more, so
					// the window IS the shell. The mock band moved into the content
					// column's own footer, which is what keeps it from running past
					// the rail.
					'grid h-screen overflow-hidden bg-background text-foreground',
					'grid-rows-[var(--row)_1fr]',
					side === 'left' ? 'grid-cols-[var(--rail)_minmax(0,1fr)]' : 'grid-cols-[minmax(0,1fr)_var(--rail)]'
				)}
			>
				{/* The rail IS the grid item, not something wrapped in one: it
				    carries the divider as a border (spec §1.3), and on a wrapper
				    that border would be dividing a box other than the surface it
				    separates. `row-span-2` travels with it — never a blank band
				    above the rail, because its own first row is what the reserve
				    and the history buttons live in (spec §1.1). Only the column
				    comes from here, because this is the only file that knows
				    which track is which. */}
				<Sidebar className={railColumn} />

				{/* The cell, not the row: `ChromeRow` renders the field itself and
				    the field is the row, so the placement has to go somewhere and
				    this is the only file that knows which column is which.

				    `bg-background` is what the field is composited against (spec
				    §6.4): the plate is `--background` at 85% over a blur, so with
				    nothing opaque behind it the row resolves against whatever the
				    window happens to be showing and the field, its lens and its
				    placeholder wash out — worst in light mode, which has the least
				    contrast to lose. */}
				<div className={cn('row-start-1 bg-background', contentColumn)}>
					<ChromeRow />
				</div>

				{/* `min-h-0` because a grid item's default `min-height: auto`
				    refuses to shrink below its content: without it a long list
				    grows this cell past the `1fr` track and scrolls the window
				    instead of the column. */}
				<main className={cn('row-start-2 flex min-h-0 flex-col bg-background', contentColumn)}>
					{/* The scroll lives on this wrapper, not on `main`: with it on the
					    cell, a footer inside would scroll away with the page instead
					    of staying pinned to the bottom of the column. */}
					<div className="min-h-0 flex-1 overflow-auto">{children}</div>
					{footer}
				</main>
			</div>
		</TooltipProvider>
	);
}
