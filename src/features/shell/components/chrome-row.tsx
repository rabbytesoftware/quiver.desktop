import type { JSX } from 'react';

import { SearchBar } from '@/features/search';

import { railOwnsControls, windowControls } from '../geometry';
import { useShellStore } from '../store';
import { WindowControls } from './window-controls';

/**
 * Row 1 of the content column. The search field IS the row rather than a
 * control sitting in one (spec §4.2), so there is no plate around it here —
 * anything wrapped about the field is a second surface, and the seam between
 * them reads as a notch cut out of the bar.
 */
export function ChromeRow(): JSX.Element {
	const side = useShellStore((s) => s.sidebarSide);

	// True in exactly one of the three combinations: macOS with the rail on the
	// right, where the lights stay on the left and the rail is no longer there
	// to hold them (spec §4.5). Both halves carry weight — `!railOwnsControls`
	// on its own is true on Windows and Linux too, where there is nothing to
	// hold in the first place.
	const controls = windowControls();
	const hostsControls = controls !== null && !railOwnsControls(side);

	// Passing `<WindowControls/>` unconditionally and letting it return `null`
	// off macOS is NOT the same thing: `leading` is a ReactNode, and an element
	// is truthy whatever it renders, so `SearchBar` would drop its leading
	// padding on every platform (spec §4.3) and the placeholder would sit flush
	// against the field's edge on the two that render no spacer at all.
	return <SearchBar leading={hostsControls ? <WindowControls /> : undefined} />;
}
