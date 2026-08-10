import type { JSX } from 'react';

import { railOwnsControls, useShellStore, WindowControls } from '@/features/shell';

import { HistoryNav } from './history-nav';

export function RailTopBar(): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);

	const reserve = railOwnsControls(side) ? <WindowControls /> : null;
	const history = <HistoryNav />;

	const [leading, trailing] = side === 'left' ? [reserve, history] : [history, reserve];

	return (
		<div data-tauri-drag-region className="flex h-(--row) items-center px-1.5">
			{leading}
			<div data-tauri-drag-region className="flex-1" />
			{trailing}
		</div>
	);
}
