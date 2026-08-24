import type { CSSProperties, JSX, ReactNode } from 'react';

import { TooltipProvider } from '@/components/ui/tooltip';

import { useShellStore } from '@/features/shell/stores/shell-store';
import { Sidebar } from '@/features/sidebar';
import { cn } from '@/lib/cn';

import { ChromeRow, useContentHoldsControls } from './chrome-row';

export interface AppShellProps {
	children: ReactNode;
	footer?: ReactNode;
}

export function AppShell({ children, footer }: AppShellProps): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);
	const width = useShellStore((s) => s.sidebarWidth);

	const holdsControls = useContentHoldsControls();

	const railColumn = side === 'left' ? 'col-start-1' : 'col-start-2';
	const contentColumn = side === 'left' ? 'col-start-2' : 'col-start-1';

	return (
		<TooltipProvider>
			<div
				data-shell
				style={{ '--rail': `${width}px` } as CSSProperties}
				className={cn(
					'grid h-screen overflow-hidden bg-background text-foreground',
					holdsControls ? 'grid-rows-[var(--row)_minmax(0,1fr)]' : 'grid-rows-[0_minmax(0,1fr)]',
					side === 'left' ? 'grid-cols-[var(--rail)_minmax(0,1fr)]' : 'grid-cols-[minmax(0,1fr)_var(--rail)]'
				)}
			>
				<Sidebar className={railColumn} />

				{holdsControls && (
					<div className={cn('row-start-1 bg-background', contentColumn)}>
						<ChromeRow />
					</div>
				)}

				<main className={cn('row-start-2 flex min-h-0 flex-col bg-background', contentColumn)}>
					{/* `scrollbar-gutter: stable` reserves the track whether or not this
					    scrolls. Without it, anything that shortens the page -- narrowing a
					    search from 70 results to 16 -- drops the scrollbar and hands its
					    17px back to the content, moving every tile sideways. The results
					    grid already pins its column *count* against that (spec 9.3.1); this
					    pins the width the count is measured against. */}
					<div className="min-h-0 flex-1 overflow-auto [scrollbar-gutter:stable]">{children}</div>
					{footer}
				</main>
			</div>
		</TooltipProvider>
	);
}
