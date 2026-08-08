import type { JSX } from 'react';

import { windowControls } from '../geometry';

/**
 * The space the window's own controls are painted into — on macOS, and only
 * there. An empty spacer, never a button: AppKit draws the three lights on top
 * of it, and this file renders no control of its own on any platform.
 *
 * `tauri.macos.conf.json` sets `titleBarStyle: "Overlay"` with `hiddenTitle`,
 * so the webview extends under the system title bar and macOS draws nothing up
 * there but the traffic lights. That buys the native look at a cost: the window
 * loses every draggable surface it had, because the frontend now owns those
 * pixels. `data-tauri-drag-region` is what hands dragging back.
 *
 * Linux and Windows get none of that, deliberately. Those three settings are
 * macOS-only — which is why they live in the macOS config file rather than the
 * shared one — so there the OS keeps drawing a real title bar above the
 * webview, with real controls, real hover glyphs and a real drag surface.
 * Nothing we could put in the grid improves on one, which is why they keep
 * theirs and `windowControls()` answers `null`. Rendering the spacer anyway
 * would open 64px of dead width inside whichever cell hosts it — a hole held
 * for three buttons that are somewhere else entirely, shoving the search field
 * sideways on the two platforms that need no reserve at all.
 *
 * That makes `decorations` load-bearing off macOS: it is what draws the ONLY
 * chrome those windows have, and `null` here plus `decorations: false` there is
 * a window with no close button, no minimise and no way to drag it — which is
 * what `tauri.windows.conf.json` produced until it was deleted. It defaults to
 * true, no platform config overrides it, and shell.test.tsx asserts that for
 * every platform.
 *
 * TRAFFIC LIGHT COUPLING: the width below and `trafficLightPosition` in
 * tauri.macos.conf.json describe the same three buttons from two different
 * sides. The buttons are 12px across, so `y: (ROW_H - 12) / 2` centres them in
 * the `--row`-tall grid row this spacer sits in, and the width is the ~64px
 * they occupy at `x: 12`. Retune `--row` without moving `y` and the lights sit
 * visibly high in the row; take the width from anywhere but `windowControls()`
 * and the two copies drift. shell.test.tsx reads both sides and fails when they
 * disagree.
 */
export function WindowControls(): JSX.Element | null {
	const controls = windowControls();
	if (controls === null) return null;

	return (
		// The width is an inline style rather than a token because it is not one:
		// it measures something AppKit draws, so it follows neither `--row` nor
		// any scale of ours (see `windowControls`).
		//
		// `shrink-0` is what keeps it exactly that wide. The spacer's hosts are
		// both flex rows containing a `flex-1` input or list, so a shrinkable
		// reserve narrows first at a small window and the lights end up painted
		// over whatever slid underneath them. `h-full` for the same reason in the
		// other axis: a zero-height drag region is a window that cannot be moved.
		<div
			// A name of its own, because `data-tauri-drag-region` no longer
			// identifies it: the whole of row 1 is a window handle now, so a query
			// for that attribute matches the rail's spacer and the search plate too.
			data-slot="window-controls"
			data-tauri-drag-region
			className="h-full shrink-0"
			style={{ width: controls.width }}
		/>
	);
}
