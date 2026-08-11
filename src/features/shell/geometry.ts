import { isMacOS } from '@/lib/platform';

export const ROW_H = 34;

export type SidebarSide = 'left' | 'right';

export interface WindowControls {
	edge: 'left' | 'right';
	kind: 'reserve';
	width: number;
}

export function windowControls(): WindowControls | null {
	return isMacOS() ? { edge: 'left', kind: 'reserve', width: 64 } : null;
}

export function railOwnsControls(side: SidebarSide): boolean {
	const controls = windowControls();
	return controls !== null && controls.edge === side;
}
