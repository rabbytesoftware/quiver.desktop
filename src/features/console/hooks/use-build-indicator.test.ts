import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { BUILD_INDICATOR_STORAGE_KEY, useBuildIndicatorStore } from '@/features/console/stores/build-indicator-store';

import { useBuildIndicatorSetting } from './use-build-indicator';

beforeEach(() => {
	localStorage.removeItem(BUILD_INDICATOR_STORAGE_KEY);
	useBuildIndicatorStore.setState({ shown: null });
});

afterEach(() => {
	vi.unstubAllEnvs();
});

function setting(channel: string) {
	vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', channel);
	return renderHook(() => useBuildIndicatorSetting());
}

describe('the build indicator setting, with no choice made', () => {
	it('is off on a stable release', () => {
		const { result } = setting('stable');
		expect(result.current).toMatchObject({ shown: false, shownByDefault: false, changed: false });
	});

	it.each([['beta'], ['hotfix'], ['nightly'], ['nightly-rolling'], ['']])('is on for %j', (channel) => {
		const { result } = setting(channel);
		expect(result.current).toMatchObject({ shown: true, shownByDefault: true, changed: false });
	});
});

describe('choosing', () => {
	it('turns it on for a stable release, and offers a reset', () => {
		const { result } = setting('stable');
		act(() => result.current.set(true));
		expect(result.current).toMatchObject({ shown: true, changed: true });
		expect(useBuildIndicatorStore.getState().shown).toBe(true);

		act(() => result.current.reset());
		expect(result.current).toMatchObject({ shown: false, changed: false });
		expect(useBuildIndicatorStore.getState().shown).toBeNull();
	});

	it('turns it off for a nightly', () => {
		const { result } = setting('nightly-rolling');
		act(() => result.current.set(false));
		expect(result.current).toMatchObject({ shown: false, changed: true });
	});

	it('stores nothing when the choice is the default, so it keeps following the channel', () => {
		const { result } = setting('stable');
		act(() => result.current.set(true));
		act(() => result.current.set(false));
		expect(useBuildIndicatorStore.getState().shown).toBeNull();
	});

	it('follows a new default when the build moves channel without a choice', () => {
		expect(setting('nightly').result.current.shown).toBe(true);
		expect(setting('stable').result.current.shown).toBe(false);
	});
});
