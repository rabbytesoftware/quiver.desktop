import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Titlebar } from './titlebar';

describe('Titlebar', () => {
	it('marks itself as a drag region', () => {
		// Without this attribute the window has no draggable surface at all —
		// "Overlay" title bar style means macOS no longer provides one.
		const { container } = render(<Titlebar />);
		expect(container.querySelector('[data-tauri-drag-region]')).not.toBeNull();
	});

	it('reserves room for the traffic lights', () => {
		// pl-20 (80px) clears the ~64px the three buttons occupy at x: 12.
		// If this padding is dropped, app chrome renders underneath them.
		const { container } = render(<Titlebar />);
		const header = container.querySelector('header');
		expect(header?.className).toContain('pl-20');
		expect(header?.className).toContain('h-12');
	});
});
