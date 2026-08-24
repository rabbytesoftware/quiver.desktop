import type { JSX } from 'react';

import type { Icon as PhosphorIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { TabsTab } from '@/components/ui/tabs';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

import { blockReselect } from '@/features/sidebar/lib/reselect';
import { cn } from '@/lib/cn';

export type NavDestination = '/' | '/remote' | '/settings';

export interface NavSegmentProps {
	to: NavDestination;
	value: string;
	active: boolean;
	icon: PhosphorIcon;
	label: string;
}

const TAB = 'flex flex-1 items-center justify-center gap-1';

function labelClass(active: boolean): string {
	return cn('hidden', active && '@[280px]:inline', '@[420px]:inline');
}

export function NavSegment({ to, value, active, icon: Icon, label }: NavSegmentProps): JSX.Element {
	const glyph = <Icon size={14} weight={active ? 'fill' : 'regular'} />;

	return (
		<TabsTab value={value} className={TAB} render={<Link to={to} onClick={blockReselect} aria-label={label} />}>
			{active ? (
				glyph
			) : (
				<Tooltip>
					<TooltipTrigger render={<span className="flex items-center justify-center" />}>
						{glyph}
					</TooltipTrigger>
					<TooltipContent side="bottom">{label}</TooltipContent>
				</Tooltip>
			)}
			<span className={labelClass(active)}>{label}</span>
		</TabsTab>
	);
}
