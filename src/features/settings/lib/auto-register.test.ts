import { describe, expect, it } from 'vitest';

import { autoRegisterPatch, autoRegisterRejection, autoRegisterResetPatch, isAutoRegisterOn } from './auto-register';

describe('isAutoRegisterOn', () => {
	it.each([
		['Fletcher is on', { manifold: { fletcher: { enabled: true } } }, true],
		['Fletcher is off', { manifold: { fletcher: { enabled: false } } }, false],
		['the config is empty', {}, false],
		['an older daemon omits the group', { manifold: { fetch_timeout: '30s' } }, false],
		['the fletcher group is empty', { manifold: { fletcher: {} } }, false],
		['manifold is null', { manifold: null }, false],
		['manifold is not an object', { manifold: 'nope' }, false],
		['fletcher is null', { manifold: { fletcher: null } }, false],
		['fletcher is not an object', { manifold: { fletcher: true } }, false],
		['the switch is not a boolean', { manifold: { fletcher: { enabled: 'yes' } } }, false],
		['search.unmarked.enabled is set but Fletcher is not', { search: { unmarked: { enabled: true } } }, false],
	])('is off/on when %s', (_name, config, expected) => {
		expect(isAutoRegisterOn(config)).toBe(expected);
	});
});

describe('autoRegisterPatch', () => {
	it('patches only manifold.fletcher.enabled, on', () => {
		expect(autoRegisterPatch(true)).toEqual({ manifold: { fletcher: { enabled: true } } });
	});

	it('patches only manifold.fletcher.enabled, off', () => {
		expect(autoRegisterPatch(false)).toEqual({ manifold: { fletcher: { enabled: false } } });
	});

	it('never touches the search section', () => {
		expect(autoRegisterPatch(true)).not.toHaveProperty('search');
		expect(autoRegisterResetPatch()).not.toHaveProperty('search');
	});

	it('round-trips through the reader', () => {
		expect(isAutoRegisterOn(autoRegisterPatch(true))).toBe(true);
		expect(isAutoRegisterOn(autoRegisterPatch(false))).toBe(false);
	});
});

describe('autoRegisterResetPatch', () => {
	it('sends null so the daemon restores its default', () => {
		expect(autoRegisterResetPatch()).toEqual({ manifold: { fletcher: { enabled: null } } });
	});
});

describe('autoRegisterRejection', () => {
	it.each([
		['the leaf', 'manifold.fletcher.enabled'],
		['the group', 'manifold.fletcher'],
		['a section stopped at by an older daemon', 'manifold'],
	])('reports a rejection of %s', (_name, key) => {
		expect(autoRegisterRejection([{ key, message: 'unknown setting' }])).toBe('unknown setting');
	});

	it('ignores rejections about other settings', () => {
		expect(
			autoRegisterRejection([
				{ key: 'logger.level', message: 'unusable log level' },
				{ key: 'manifold.fetch_timeout', message: 'bad' },
				{ key: 'search.unmarked.min_stars', message: 'bad' },
			])
		).toBeUndefined();
	});

	it('returns nothing when nothing was rejected', () => {
		expect(autoRegisterRejection([])).toBeUndefined();
	});
});
