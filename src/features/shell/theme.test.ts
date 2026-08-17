import { beforeEach, describe, expect, it, vi } from 'vitest';

import { installThemeSync, THEME_STORAGE_KEY, useThemeStore } from './theme';

function stubMatchMedia(matches: boolean) {
	const listeners = new Set<() => void>();
	vi.stubGlobal('matchMedia', () => ({
		matches,
		addEventListener: (_: string, cb: () => void) => listeners.add(cb),
		removeEventListener: (_: string, cb: () => void) => listeners.delete(cb),
	}));
	return { fire: () => listeners.forEach((cb) => cb()) };
}

beforeEach(() => {
	localStorage.removeItem(THEME_STORAGE_KEY);
	document.documentElement.classList.remove('dark');
	useThemeStore.setState({ preference: 'system' });
});

describe('theme', () => {
	it('defaults to following the system', () => {
		expect(useThemeStore.getState().preference).toBe('system');
	});

	it('adds the dark class when the preference is dark', () => {
		stubMatchMedia(false);
		const dispose = installThemeSync();
		useThemeStore.getState().setPreference('dark');
		expect(document.documentElement.classList.contains('dark')).toBe(true);
		dispose();
	});

	it('removes the dark class when the preference is light, whatever the system says', () => {
		stubMatchMedia(true);
		const dispose = installThemeSync();
		useThemeStore.getState().setPreference('light');
		expect(document.documentElement.classList.contains('dark')).toBe(false);
		dispose();
	});

	it('follows the system while the preference is system', () => {
		stubMatchMedia(true);
		const dispose = installThemeSync();
		expect(document.documentElement.classList.contains('dark')).toBe(true);
		dispose();
	});

	it('stops following the system once an explicit choice is made', () => {
		const media = stubMatchMedia(true);
		const dispose = installThemeSync();
		useThemeStore.getState().setPreference('light');
		media.fire();
		expect(document.documentElement.classList.contains('dark')).toBe(false);
		dispose();
	});

	it('stops reacting after disposal', () => {
		stubMatchMedia(false);
		const dispose = installThemeSync();
		dispose();
		useThemeStore.getState().setPreference('dark');
		expect(document.documentElement.classList.contains('dark')).toBe(false);
		dispose();
	});
});
