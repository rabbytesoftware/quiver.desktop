import type { JSX } from 'react';

import { WindowControls } from '@/features/shell/components/window-controls';
import { railOwnsControls } from '@/features/shell/lib/geometry';
import { useShellStore } from '@/features/shell/stores/shell-store';
import { HistoryNav } from '@/features/sidebar/components/nav/history-nav';

const CONTROLS = <WindowControls />;
const HISTORY = <HistoryNav />;

export function RailTopBar(): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);

	const reserve = railOwnsControls(side) ? CONTROLS : null;
	const history = HISTORY;

	const [leading, trailing] = side === 'left' ? [reserve, history] : [history, reserve];

	return (
		<div data-tauri-drag-region className="flex h-(--row) items-center px-1.5">
			{leading}
			<div data-tauri-drag-region className="flex-1" />
			{trailing}
		</div>
	);
}
