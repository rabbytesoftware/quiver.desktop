import type { JSX } from 'react';

import { railOwnsControls, windowControls } from '../geometry';
import { useShellStore } from '../store';
import { WindowControls } from './window-controls';

export function useContentHoldsControls(): boolean {
	const side = useShellStore((s) => s.sidebarSide);
	return windowControls() !== null && !railOwnsControls(side);
}

export function ChromeRow(): JSX.Element {
	return (
		<div data-tauri-drag-region className="flex h-(--row) items-center">
			<WindowControls />
		</div>
	);
}
