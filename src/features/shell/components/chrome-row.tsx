import type { JSX } from 'react';

import { railOwnsControls, windowControls } from '../geometry';
import { useShellStore } from '../store';
import { WindowControls } from './window-controls';

/**
 * Whether the content column has to hold the macOS traffic-light reserve.
 *
 * True in exactly one of the three combinations: macOS with the rail on the
 * RIGHT, where the lights stay on the left and the rail is no longer there to
 * hold them (spec §4.5). Both halves carry weight — `!railOwnsControls` alone
 * is true on Windows and Linux too, where there is nothing to hold.
 *
 * Exported because `AppShell` needs the same answer to decide whether row 1
 * exists at all, and two copies of this rule would disagree on the day either
 * moves.
 */
export function useContentHoldsControls(): boolean {
	const side = useShellStore((s) => s.sidebarSide);
	return windowControls() !== null && !railOwnsControls(side);
}

/**
 * Row 1 of the content column, and now ONLY the traffic-light reserve.
 *
 * The search field used to be this row — it is in the rail now, above the
 * changer — which leaves the row with one job and only on one platform in one
 * orientation. `AppShell` collapses the track to zero everywhere else rather
 * than painting an empty 34px band across the top of the content.
 */
export function ChromeRow(): JSX.Element {
	return (
		// A window handle, as the row it replaces was: `titleBarStyle: "Overlay"`
		// leaves macOS drawing no draggable surface of its own, and this strip is
		// the only chrome above the content on the one layout that renders it.
		<div data-tauri-drag-region className="flex h-(--row) items-center">
			<WindowControls />
		</div>
	);
}
