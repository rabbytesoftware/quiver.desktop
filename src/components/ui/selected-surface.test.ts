import { describe, expect, it } from 'vitest';

import { SELECTED_SURFACE, SELECTED_SURFACE_ACTIVE } from './selected-surface';

const PREFIX = 'data-[status=active]:';

const classes = (value: string): string[] => value.split(/\s+/).filter(Boolean).sort();

describe('selected-surface', () => {
	/**
	 * Tailwind scans for literal class names, so a variant prefix applied with
	 * .map() at runtime is never generated. The two strings therefore have to be
	 * written out separately, and this is what keeps the rail's selected row and
	 * the changer's indicator from drifting apart.
	 */
	it('describes one surface in both forms', () => {
		const active = classes(SELECTED_SURFACE_ACTIVE).map((cls) => cls.replace(PREFIX, ''));

		// `border` alone is the only difference, and it is deliberate: ROW_BASE
		// puts the 1px on EVERY row so the list cannot reflow as the selection
		// moves. The indicator has no such base to inherit it from.
		expect(classes(SELECTED_SURFACE)).toEqual([...active, 'border'].sort());
	});

	it('carries the prefix on every class, not just the first', () => {
		for (const cls of classes(SELECTED_SURFACE_ACTIVE)) {
			expect(cls.startsWith(PREFIX)).toBe(true);
		}
	});
});
