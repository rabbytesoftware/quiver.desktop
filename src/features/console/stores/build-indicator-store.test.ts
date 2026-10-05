import { beforeEach, describe, expect, it } from 'vitest';

import { BUILD_INDICATOR_STORAGE_KEY, normaliseShown, useBuildIndicatorStore } from './build-indicator-store';

beforeEach(() => {
	localStorage.removeItem(BUILD_INDICATOR_STORAGE_KEY);
	useBuildIndicatorStore.setState({ shown: null });
});

describe('the build indicator preference', () => {
	it('starts as no choice, so the build’s channel decides', () => {
		expect(useBuildIndicatorStore.getState().shown).toBeNull();
	});

	it('remembers an explicit choice either way, and forgets it on reset', () => {
		useBuildIndicatorStore.getState().setShown(false);
		expect(useBuildIndicatorStore.getState().shown).toBe(false);
		useBuildIndicatorStore.getState().setShown(true);
		expect(useBuildIndicatorStore.getState().shown).toBe(true);
		useBuildIndicatorStore.getState().reset();
		expect(useBuildIndicatorStore.getState().shown).toBeNull();
	});

	it('writes the choice under its own key, and no choice as null', () => {
		useBuildIndicatorStore.getState().setShown(true);
		expect(JSON.parse(localStorage.getItem(BUILD_INDICATOR_STORAGE_KEY) ?? '{}').state).toEqual({ shown: true });
		useBuildIndicatorStore.getState().reset();
		expect(JSON.parse(localStorage.getItem(BUILD_INDICATOR_STORAGE_KEY) ?? '{}').state).toEqual({ shown: null });
	});

	it('reads back a saved choice after a reload', async () => {
		localStorage.setItem(BUILD_INDICATOR_STORAGE_KEY, JSON.stringify({ state: { shown: false }, version: 0 }));
		await useBuildIndicatorStore.persist.rehydrate();
		expect(useBuildIndicatorStore.getState().shown).toBe(false);
	});

	it.each([['yes'], [1], [{}], [undefined]])('treats %j on disk as no choice', async (junk) => {
		localStorage.setItem(BUILD_INDICATOR_STORAGE_KEY, JSON.stringify({ state: { shown: junk }, version: 0 }));
		await useBuildIndicatorStore.persist.rehydrate();
		expect(useBuildIndicatorStore.getState().shown).toBeNull();
	});

	it('normalises only booleans', () => {
		expect(normaliseShown(true)).toBe(true);
		expect(normaliseShown(false)).toBe(false);
		expect(normaliseShown(null)).toBeNull();
		expect(normaliseShown('false')).toBeNull();
	});
});
