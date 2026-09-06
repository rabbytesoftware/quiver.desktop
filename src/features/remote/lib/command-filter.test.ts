import { describe, expect, it } from 'vitest';

import type { ConnectionConfig } from '@/domain/connection';

import { clampIndex, filterConnections } from './command-filter';

const local: ConnectionConfig = { id: 'local', name: 'Local', kind: 'local', api_version: 'v0' };
const homeLab: ConnectionConfig = { id: 'home-lab', name: 'Home Lab', kind: 'remote', api_version: 'v0' };

describe('filterConnections', () => {
	it('returns every connection when the query is blank', () => {
		expect(filterConnections([local, homeLab], '')).toEqual([local, homeLab]);
	});

	it('matches case-insensitively on a substring of the name', () => {
		expect(filterConnections([local, homeLab], 'home')).toEqual([homeLab]);
	});

	it('returns an empty list when nothing matches', () => {
		expect(filterConnections([local, homeLab], 'garage')).toEqual([]);
	});

	it('ignores leading and trailing whitespace in the query', () => {
		expect(filterConnections([local, homeLab], '  local  ')).toEqual([local]);
	});
});

describe('clampIndex', () => {
	it('wraps forward past the last index back to zero', () => {
		expect(clampIndex(2, 2)).toBe(0);
	});

	it('wraps backward past zero to the last index', () => {
		expect(clampIndex(-1, 2)).toBe(1);
	});

	it('leaves an in-range index untouched', () => {
		expect(clampIndex(1, 3)).toBe(1);
	});

	it('is zero for an empty list', () => {
		expect(clampIndex(4, 0)).toBe(0);
	});
});
