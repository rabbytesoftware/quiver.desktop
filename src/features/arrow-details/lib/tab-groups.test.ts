import { describe, expect, it } from 'vitest';

import { groupTabs } from './tab-groups';

interface Entry {
	value: string;
	groupable: boolean;
}

function entry(value: string, groupable: boolean): Entry {
	return { value, groupable };
}

describe('groupTabs', () => {
	it('pairs two consecutive groupable entries', () => {
		const groups = groupTabs([entry('a', true), entry('b', true)]);
		expect(groups).toEqual([[entry('a', true), entry('b', true)]]);
	});

	it('pairs four groupable entries into two consecutive pairs', () => {
		const groups = groupTabs([entry('a', true), entry('b', true), entry('c', true), entry('d', true)]);
		expect(groups.map((g) => g.map((e) => e.value))).toEqual([
			['a', 'b'],
			['c', 'd'],
		]);
	});

	it('leaves a non-groupable entry alone, even when the next one is groupable', () => {
		const groups = groupTabs([entry('readme', false), entry('a', true), entry('b', true)]);
		expect(groups.map((g) => g.map((e) => e.value))).toEqual([['readme'], ['a', 'b']]);
	});

	it('leaves an odd leftover groupable entry alone', () => {
		const groups = groupTabs([entry('a', true), entry('b', true), entry('c', true)]);
		expect(groups.map((g) => g.map((e) => e.value))).toEqual([['a', 'b'], ['c']]);
	});

	it('leaves a lone groupable entry alone when there is nothing to pair it with', () => {
		const groups = groupTabs([entry('readme', false), entry('a', true)]);
		expect(groups.map((g) => g.map((e) => e.value))).toEqual([['readme'], ['a']]);
	});

	it('never pairs two non-groupable entries', () => {
		const groups = groupTabs([entry('a', false), entry('b', false)]);
		expect(groups.map((g) => g.map((e) => e.value))).toEqual([['a'], ['b']]);
	});

	it('returns an empty list for no entries', () => {
		expect(groupTabs([])).toEqual([]);
	});

	it('returns a single solo group for one entry', () => {
		expect(groupTabs([entry('a', true)]).map((g) => g.map((e) => e.value))).toEqual([['a']]);
	});
});
