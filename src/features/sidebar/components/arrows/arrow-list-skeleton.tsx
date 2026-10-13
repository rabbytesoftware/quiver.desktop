import type { JSX } from 'react';

import { ROW_BASE, ROW_GLYPH_BOX } from '@/features/sidebar/lib/row-base';
import { cn } from '@/lib/cn';

const ROW = cn(ROW_BASE, 'border-transparent animate-pulse cursor-default');

const PLACEHOLDERS = ['a', 'b', 'c', 'd', 'e', 'f'] as const;

const WIDTHS: Record<(typeof PLACEHOLDERS)[number], string> = {
	a: 'w-28',
	b: 'w-20',
	c: 'w-32',
	d: 'w-24',
	e: 'w-28',
	f: 'w-16',
};

export function ArrowListSkeleton({ label }: { label: string }): JSX.Element {
	return (
		<div
			aria-busy="true"
			aria-label={label}
			className="flex flex-col"
			data-slot="arrow-list-skeleton"
			role="status"
		>
			{PLACEHOLDERS.map((id) => (
				<div aria-hidden="true" className={ROW} key={id}>
					<span className={cn(ROW_GLYPH_BOX, 'rounded-sm bg-sidebar-element-idle')} />
					<span className={cn('h-2.5 rounded-sm bg-sidebar-element-idle', WIDTHS[id])} />
				</div>
			))}
		</div>
	);
}
