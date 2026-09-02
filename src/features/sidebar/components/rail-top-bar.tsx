import type { JSX } from 'react';

import { ConnectionSwitcher } from '@/features/remote';
import { WindowControls } from '@/features/shell/components/window-controls';
import { railOwnsControls } from '@/features/shell/lib/geometry';
import { useShellStore } from '@/features/shell/stores/shell-store';
import { HistoryNav } from '@/features/sidebar/components/nav/history-nav';

const CONTROLS = <WindowControls />;
// Fragments, not a wrapping div: the pair travels together without adding a
// DOM level between it and the row -- `HistoryNav`'s own wrapper stays a
// direct child of it. The arrows always sit closest to the content edge
// (asserted by "faces the history buttons at the content" below), with the
// switcher on the outer, window-edge side of them -- so which one comes
// first has to flip with `side`, since `[leading, trailing]` below always
// puts this pair on the content-facing slot, not a fixed screen side.
const HISTORY_LEFT = (
	<>
		<ConnectionSwitcher />
		<HistoryNav />
	</>
);
const HISTORY_RIGHT = (
	<>
		<HistoryNav />
		<ConnectionSwitcher />
	</>
);

export function RailTopBar(): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);

	const reserve = railOwnsControls(side) ? CONTROLS : null;
	const history = side === 'left' ? HISTORY_LEFT : HISTORY_RIGHT;

	const [leading, trailing] = side === 'left' ? [reserve, history] : [history, reserve];

	return (
		<div data-tauri-drag-region className="flex h-(--row) items-center px-1.5">
			{leading}
			<div data-tauri-drag-region className="flex-1" />
			{trailing}
		</div>
	);
}
