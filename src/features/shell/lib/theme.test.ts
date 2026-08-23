import { beforeEach, describe, expect, it, vi } from 'vitest';

import { installThemeSync, normalisePreference, THEME_STORAGE_KEY, useThemeStore } from './theme';
// Relative, not `@/`: the alias only reaches into `src`, and the boot script
// under test lives in the repo-root `index.html`.
import indexHtml from '../../../../index.html?raw';

function stubMatchMedia(initialMatches: boolean) {
	const listeners = new Set<() => void>();
	let matches = initialMatches;
	vi.stubGlobal('matchMedia', () => ({
		get matches() {
			return matches;
		},
		addEventListener: (_: string, cb: () => void) => listeners.add(cb),
		removeEventListener: (_: string, cb: () => void) => listeners.delete(cb),
	}));
	return {
		fire: () => listeners.forEach((cb) => cb()),
		setMatches: (value: boolean) => {
			matches = value;
		},
	};
}

beforeEach(() => {
	localStorage.removeItem(THEME_STORAGE_KEY);
	document.documentElement.classList.remove('dark');
	useThemeStore.setState({ preference: 'system' });
});

describe('normalisePreference', () => {
	it('keeps the three valid preferences', () => {
		expect(normalisePreference('light')).toBe('light');
		expect(normalisePreference('dark')).toBe('dark');
		expect(normalisePreference('system')).toBe('system');
	});

	it.each([['de'], [null], [undefined], [42], [{ preference: 'dark' }]])('rejects %o', (value) => {
		expect(normalisePreference(value)).toBe('system');
	});
});

describe('theme rehydration', () => {
	it('reads a stored choice back on rehydration', async () => {
		localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify({ state: { preference: 'dark' }, version: 0 }));
		await useThemeStore.persist.rehydrate();

		expect(useThemeStore.getState().preference).toBe('dark');
	});

	it('degrades a stored preference that is no longer valid back to "system"', async () => {
		localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify({ state: { preference: 'invalid' }, version: 0 }));
		await useThemeStore.persist.rehydrate();

		expect(useThemeStore.getState().preference).toBe('system');
	});
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
		const media = stubMatchMedia(false);
		const dispose = installThemeSync();
		expect(document.documentElement.classList.contains('dark')).toBe(false);
		media.setMatches(true);
		media.fire();
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
		const media = stubMatchMedia(false);
		const dispose = installThemeSync();
		dispose();
		media.setMatches(true);
		media.fire();
		expect(document.documentElement.classList.contains('dark')).toBe(false);
	});

	it('survives when matchMedia is not available', () => {
		vi.stubGlobal('matchMedia', undefined);
		const dispose = installThemeSync();
		expect(document.documentElement.classList.contains('dark')).toBe(false);
		useThemeStore.getState().setPreference('dark');
		expect(document.documentElement.classList.contains('dark')).toBe(true);
		dispose();
		expect(document.documentElement.classList.contains('dark')).toBe(true);
	});
});

describe("index.html's pre-paint script", () => {
	// index.html can't import THEME_STORAGE_KEY — it runs before any module
	// graph exists, so the key is a hardcoded literal there. Nothing ties the
	// two together, so renaming the constant here would silently desync the
	// pre-paint script (reverting it to preference-blind behaviour) without
	// a single test noticing. Reading the file straight off disk is the only
	// way to keep them honest.
	it('reads the same localStorage key the theme store persists to', () => {
		expect(indexHtml).toContain(`localStorage.getItem('${THEME_STORAGE_KEY}')`);
	});
});
