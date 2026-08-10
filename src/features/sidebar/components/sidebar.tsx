import type { JSX } from 'react';

import { SearchBar } from '@/features/search';
import { useShellStore } from '@/features/shell';
import { cn } from '@/lib/cn';

import { ArrowList } from './arrow-list';
import { PrimaryNav } from './primary-nav';
import { RailTopBar } from './rail-top-bar';
import { ResizeHandle } from './resize-handle';

export interface SidebarProps {
	className?: string;
}

export function Sidebar({ className }: SidebarProps): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);

	return (
		<div
			data-slot="sidebar"
			className={cn(
				'relative row-span-2 flex min-h-0 flex-col',
				'bg-sidebar text-sidebar-foreground border-sidebar-border',
				side === 'left' ? 'border-r' : 'border-l',
				className
			)}
		>
			<RailTopBar />
			<div className="flex shrink-0 flex-col gap-1.5 px-2 pt-0.5 pb-1">
				<SearchBar />
				<PrimaryNav />
			</div>
			<ArrowList />
			<ResizeHandle />
		</div>
	);
}
