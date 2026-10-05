import { describe, expect, it } from 'vitest';

import { appendCapped } from './ring';

describe('appendCapped', () => {
	it('appends below the cap', () => expect(appendCapped([1, 2], [3], 5)).toEqual([1, 2, 3]));

	it('keeps only the newest items at the cap', () => {
		expect(appendCapped([1, 2, 3], [4, 5], 4)).toEqual([2, 3, 4, 5]);
	});

	it('keeps the tail of an addition larger than the cap', () => {
		expect(appendCapped([1], [2, 3, 4, 5, 6], 3)).toEqual([4, 5, 6]);
	});

	it('trims an oversized list even when nothing is added', () => {
		expect(appendCapped([1, 2, 3, 4], [], 2)).toEqual([3, 4]);
		expect(appendCapped([1, 2], [], 5)).toEqual([1, 2]);
	});

	it('works from empty', () => expect(appendCapped([], [1, 2], 5)).toEqual([1, 2]));

	it('never mutates its inputs', () => {
		const list = Object.freeze([1, 2, 3]);
		const add = Object.freeze([4]);
		expect(appendCapped(list, add, 3)).toEqual([2, 3, 4]);
		expect(list).toEqual([1, 2, 3]);
	});

	it('holds 5000 under a flood', () => {
		let list: number[] = [];
		for (let i = 0; i < 12; i++)
			list = appendCapped(
				list,
				Array.from({ length: 1000 }, (_, j) => i * 1000 + j),
				5000
			);
		expect(list).toHaveLength(5000);
		expect(list[0]).toBe(7000);
		expect(list[4999]).toBe(11_999);
	});
});
