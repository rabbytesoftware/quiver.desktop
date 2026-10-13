import { describe, expect, it } from 'vitest';

import { flattenConfig, formatConfigValue, toConfigDisplayRows } from './config';

describe('flattenConfig', () => {
	it('keeps a flat section as it is', () => {
		expect(flattenConfig({ per_provider_limit: 25, provider_timeout: '10s' })).toEqual([
			{ key: 'per_provider_limit', value: 25 },
			{ key: 'provider_timeout', value: '10s' },
		]);
	});

	it('flattens a nested object into dotted keys', () => {
		expect(flattenConfig({ fetch_concurrency: 8, unmarked: { enabled: true, min_stars: 50 } })).toEqual([
			{ key: 'fetch_concurrency', value: 8 },
			{ key: 'unmarked.enabled', value: true },
			{ key: 'unmarked.min_stars', value: 50 },
		]);
	});

	it('flattens objects nested more than one level deep', () => {
		expect(flattenConfig({ a: { b: { c: { d: 1 } }, e: false } })).toEqual([
			{ key: 'a.b.c.d', value: 1 },
			{ key: 'a.e', value: false },
		]);
	});

	it('keeps an array as one leaf instead of indexing into it', () => {
		expect(flattenConfig({ hosts: ['github.com', 'gitlab.com'] })).toEqual([
			{ key: 'hosts', value: ['github.com', 'gitlab.com'] },
		]);
	});

	it('keeps null as a leaf', () => {
		expect(flattenConfig({ unmarked: { min_stars: null }, channel: null })).toEqual([
			{ key: 'unmarked.min_stars', value: null },
			{ key: 'channel', value: null },
		]);
	});

	it('keeps an empty object as a leaf so the group does not vanish', () => {
		expect(flattenConfig({ unmarked: {} })).toEqual([{ key: 'unmarked', value: {} }]);
	});

	it('returns nothing for an empty section', () => {
		expect(flattenConfig({})).toEqual([]);
	});
});

describe('formatConfigValue', () => {
	it.each([
		['a number', 25, '25'],
		['a string', '10s', '10s'],
		['true', true, 'true'],
		['false', false, 'false'],
		['null', null, 'null'],
		['undefined', undefined, 'undefined'],
		['an empty object', {}, '{}'],
		['an empty array', [], '[]'],
		['an array of scalars', ['a', 2, true], 'a, 2, true'],
		['an array holding null', ['a', null], 'a, null'],
		['an array of objects', [{ a: 1 }, { b: 2 }], '{"a":1}, {"b":2}'],
	])('formats %s', (_name, value, expected) => {
		expect(formatConfigValue(value)).toBe(expected);
	});
});

describe('toConfigDisplayRows', () => {
	it('never renders [object Object] for a nested section', () => {
		const rows = toConfigDisplayRows({
			per_provider_limit: 25,
			fetch_concurrency: 8,
			provider_timeout: '10s',
			unmarked: { min_stars: 50, probe_limit: 10 },
			auto_retry: { enabled: true, retries: 3 },
		});
		expect(rows).toEqual([
			{ key: 'per_provider_limit', value: '25' },
			{ key: 'fetch_concurrency', value: '8' },
			{ key: 'provider_timeout', value: '10s' },
			{ key: 'unmarked.min_stars', value: '50' },
			{ key: 'unmarked.probe_limit', value: '10' },
			{ key: 'auto_retry.enabled', value: 'true' },
			{ key: 'auto_retry.retries', value: '3' },
		]);
		expect(rows.some((row) => row.value.includes('[object Object]'))).toBe(false);
	});
});
