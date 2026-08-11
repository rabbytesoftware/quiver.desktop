import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { restoreLanguages, speaking } from '@/__mocks__/navigator-language';

import { installLocaleSync } from './bootstrap';
import { currentLocale, LOCALE_STORAGE_KEY, localeOf, normalisePreference, t, useLocaleStore } from './store';

beforeEach(() => {
	useLocaleStore.setState({ preference: 'system', detected: 'en' });
	localStorage.removeItem(LOCALE_STORAGE_KEY);
});

afterEach(() => {
	restoreLanguages();
	vi.unstubAllEnvs();
});

describe('the saved preference', () => {
	it('follows the system out of the box, rather than pinning a language', () => {
		expect(useLocaleStore.getState().preference).toBe('system');
	});

	it('writes through to the namespaced key so a reload reads it back', () => {
		useLocaleStore.getState().setPreference('en');

		const persisted = JSON.parse(localStorage.getItem(LOCALE_STORAGE_KEY) ?? '{}') as {
			state?: { preference?: string; detected?: string };
		};
		expect(persisted.state?.preference).toBe('en');
		expect(persisted.state?.detected).toBeUndefined();
	});

	it('reads a stored choice back on rehydration', async () => {
		localStorage.setItem(LOCALE_STORAGE_KEY, JSON.stringify({ state: { preference: 'en' }, version: 0 }));
		await useLocaleStore.persist.rehydrate();

		expect(useLocaleStore.getState().preference).toBe('en');
	});

	it('degrades a stored locale that no longer ships back to "system"', async () => {
		localStorage.setItem(LOCALE_STORAGE_KEY, JSON.stringify({ state: { preference: 'de' }, version: 0 }));
		await useLocaleStore.persist.rehydrate();

		expect(useLocaleStore.getState().preference).toBe('system');
	});
});

describe('normalisePreference', () => {
	it('keeps the two things that are real', () => {
		expect(normalisePreference('system')).toBe('system');
		expect(normalisePreference('en')).toBe('en');
	});

	it.each([['de'], [null], [undefined], [42], [{ preference: 'en' }]])('rejects %o', (value) => {
		expect(normalisePreference(value)).toBe('system');
	});
});

describe('the locale in force', () => {
	it('is the detected one while the preference is "system"', () => {
		expect(localeOf({ preference: 'system', detected: 'en' })).toBe('en');
	});

	it('is the chosen one once there is a choice', () => {
		expect(localeOf({ preference: 'en', detected: 'en' })).toBe('en');
	});

	it('is the forced one when the environment says so, over both', () => {
		vi.stubEnv('VITE_QUIVER_LOCALE', 'en');
		expect(localeOf({ preference: 'system', detected: 'en' })).toBe('en');
	});

	it('is readable outside React', () => {
		expect(currentLocale()).toBe('en');
	});
});

describe('re-detection', () => {
	it('re-reads the system list rather than trusting the value from boot', () => {
		speaking(['fr-FR', 'fr']);
		useLocaleStore.getState().redetect();

		expect(useLocaleStore.getState().detected).toBe('en');
	});
});

describe('translating outside React', () => {
	it('resolves against the locale in force at call time', () => {
		expect(t('settings.title')).toBe('Settings');
		expect(t('settings.version.remaining', { count: 1 })).toBe('1 more tap…');
	});
});

describe('installLocaleSync', () => {
	it('stamps the document language, which index.html can only hard-code', () => {
		const teardown = installLocaleSync();
		expect(document.documentElement.lang).toBe('en');
		teardown();
	});

	it('re-detects when the OS language changes under the running app', () => {
		const teardown = installLocaleSync();
		const redetect = vi.spyOn(useLocaleStore.getState(), 'redetect');

		speaking(['fr-FR']);
		window.dispatchEvent(new Event('languagechange'));
		expect(redetect).toHaveBeenCalled();

		teardown();
		redetect.mockClear();
		window.dispatchEvent(new Event('languagechange'));
		expect(redetect).not.toHaveBeenCalled();
	});

	it('restamps the document on every locale-bearing change, and stops on teardown', () => {
		const teardown = installLocaleSync();

		document.documentElement.lang = 'xx';
		useLocaleStore.getState().setPreference('en');
		expect(document.documentElement.lang).toBe('en');

		teardown();
		document.documentElement.lang = 'xx';
		useLocaleStore.getState().setPreference('system');
		expect(document.documentElement.lang).toBe('xx');
	});
});
