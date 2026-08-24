import type { JSX } from 'react';

import { windowControls } from '@/features/shell/lib/geometry';

export function WindowControls(): JSX.Element | null {
	const controls = windowControls();
	if (controls === null) return null;

	return (
		<div
			data-slot="window-controls"
			data-tauri-drag-region
			className="h-full shrink-0"
			style={{ width: controls.width }}
		/>
	);
}
