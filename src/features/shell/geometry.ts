// The shell's geometry, and the one place the platform question is asked.
//
// macOS is frameless — `titleBarStyle: "Overlay"` with `hiddenTitle` hides the
// bar and keeps the real traffic lights — so the frontend owns the pixels the
// buttons are painted over. Windows and Linux keep the title bar the OS draws,
// because hiding it there takes the buttons with it and hand-drawn
// replacements owe Segoe Fluent glyphs, snap layouts, hover and inactive
// states forever.

import { isMacOS } from '@/lib/platform';

/**
 * Row height, in CSS px — the search field, every nav segment, every arrow row
 * and both chrome rows.
 *
 * `--row` in index.css and `trafficLightPosition.y` in tauri.macos.conf.json
 * are the same number seen from a stylesheet and from a JSON file that cannot
 * read one. Neither can import this, so moving it here alone leaves the lights
 * off-centre in the row; `titlebar.test.tsx` is what notices.
 */
export const ROW_H = 34;

/** Which edge of the window the rail is docked to. */
export type SidebarSide = 'left' | 'right';

export interface WindowControls {
	edge: 'left' | 'right';
	/** The OS paints its buttons over an empty spacer. We never render any. */
	kind: 'reserve';
	width: number;
}

/**
 * Where the window's controls land, or `null` when the OS is drawing them
 * somewhere we cannot see.
 *
 * `null` rather than a `kind: 'none'` variant: off macOS there is no edge, no
 * width and nothing to place, and a variant carrying three meaningless fields
 * invites a caller to lay out a 64px hole that never gets painted into. Two
 * call sites take the null check instead.
 *
 * The 64 is what the three 12px lights plus their gaps occupy at `x: 12`. It
 * is a reserve for something AppKit draws, not a measurement of anything we
 * control, so it does not follow from `ROW_H` or any token.
 */
export function windowControls(): WindowControls | null {
	return isMacOS() ? { edge: 'left', kind: 'reserve', width: 64 } : null;
}

/**
 * Does the rail hold the reserve, or does the chrome row?
 *
 * `side` is a parameter rather than a read of the shell store so this stays
 * pure and every cell of the platform-by-side table is testable without a
 * window. The false-on-macOS case is the interesting one: with the rail on the
 * right, the lights are still on the left, so the reserve belongs to the
 * content column instead.
 */
export function railOwnsControls(side: SidebarSide): boolean {
	const controls = windowControls();
	return controls !== null && controls.edge === side;
}
