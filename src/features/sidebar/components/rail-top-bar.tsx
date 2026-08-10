import type { JSX } from 'react';

import { railOwnsControls, useShellStore, WindowControls } from '@/features/shell';

import { HistoryNav } from './history-nav';

/**
 * Row 1 of the rail — the reason there is never a blank band above the rail
 * (spec §1.1). The row is held open because something is sitting in it.
 *
 * THE WINDOW'S EDGE BELONGS TO THE OS, THE INTERIOR BELONGS TO THE APP
 * (spec §5.8). The traffic-light reserve hugs the window; back and forward face
 * the content column, so the two never share an edge. Put them on the same one
 * and macOS paints its three lights over the history buttons, which are then
 * unreachable for the rest of the window's life — and there is no platform in
 * the test matrix where that shows up as anything but "the buttons are gone".
 */
export function RailTopBar(): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);

	// `railOwnsControls(side)` and not `windowControls() !== null`: with the rail
	// on the right the lights stay on the LEFT, so the chrome row holds the
	// reserve instead (spec §4.5) and holding it here as well opens 64px twice
	// in one window for one set of buttons. `WindowControls` also returns null
	// off macOS, but an element is truthy whatever it renders, so the guard has
	// to happen before it is placed.
	const reserve = railOwnsControls(side) ? <WindowControls /> : null;
	const history = <HistoryNav />;

	// The whole of the rule, as one ternary. An `edge` prop or a slot map would
	// read as more general and say less: which end is the window's and which is
	// the app's is the only thing this component decides.
	const [leading, trailing] = side === 'left' ? [reserve, history] : [history, reserve];

	return (
		// `data-tauri-drag-region` makes the whole strip a window handle. macOS
		// hides its title bar under `titleBarStyle: "Overlay"` and takes every
		// draggable surface with it, so without this the top of the window is
		// dead and the only way to move it is the 64px the lights sit on.
		//
		// Tauri dispatches on the event TARGET, so the two history buttons stay
		// clickable — they are their own targets and carry no such attribute.
		//
		// `px-1.5` and not `px-2`, because the buttons carry a 2px margin of
		// their own: 6 + 2 lands their outer edge on the same 8px inset the
		// search field and the changer use. Without it the arrows sat 6px further
		// out than everything below them, which is exactly the kind of thing that
		// reads as "off" without being obvious why.
		<div data-tauri-drag-region className="flex h-(--row) items-center px-1.5">
			{leading}
			{/* Holds the two ends apart at every rail width. Inherits the drag
			    region from the row rather than declaring its own — the attribute
			    is not needed on a child for a click there to hit the parent,
			    because this element is not a target of its own.

			    Holds the two ends apart at every rail width. `justify-between`
			    cannot: off macOS there is no reserve at all, and with a single
			    child it collapses to `flex-start` — which puts the history
			    buttons on the window's edge, the one place §5.8 says they never
			    go, on the two platforms nobody develops on. */}
			<div data-tauri-drag-region className="flex-1" />
			{trailing}
		</div>
	);
}
