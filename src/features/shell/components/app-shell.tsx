import type { CSSProperties, JSX, ReactNode } from 'react';

import { TooltipProvider } from '@/components/ui/tooltip';

import { Sidebar } from '@/features/sidebar';
import { cn } from '@/lib/cn';

import { useShellStore } from '../store';
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
					<div className="min-h-0 flex-1 overflow-auto">{children}</div>
					{footer}
				</main>
			</div>
		</TooltipProvider>
	);
}
