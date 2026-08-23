import { describe, expect, it } from 'vitest';

import { columnCap, columnRule } from './columns';

describe('columnCap', () => {
	it('spends the frame on the art when the answer is thin', () => {
		expect(columnCap(1)).toBe(2);
		expect(columnCap(3)).toBe(2);
	});

	it('opens up as the answer grows', () => {
		expect(columnCap(4)).toBe(4);
		expect(columnCap(8)).toBe(4);
		expect(columnCap(9)).toBe(5);
		expect(columnCap(100)).toBe(5);
	});
});

describe('columnRule', () => {
	it('caps the count rather than fixing a tile width', () => {
		// A tile floor made the outcome depend on the exact grid width, because
		// auto-fill divides (width + gap) by (floor + gap) and the result lands
		// either side of a whole number.
		expect(columnRule(12)).toBe('repeat(auto-fill, minmax(max(190px, calc((100% - 48px) / 5)), 1fr))');
		expect(columnRule(2)).toBe('repeat(auto-fill, minmax(max(190px, calc((100% - 12px) / 2)), 1fr))');
	});

	it('never lets a tile go below the minimum, which is what makes it narrow on its own', () => {
		for (const total of [1, 4, 9, 40]) expect(columnRule(total)).toContain('max(190px,');
	});

	it('subtracts exactly the gaps between the capped columns', () => {
		expect(columnRule(6)).toContain('100% - 36px');
	});
});
