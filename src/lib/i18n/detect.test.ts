import { afterEach, describe, expect, it, vi } from 'vitest';

import { restoreLanguages, speaking } from '@/__mocks__/navigator-language';

import { detectLocale, localeForcedByEnv, matchLocale, systemLocales } from './detect';

afterEach(() => {
	restoreLanguages();
	vi.unstubAllEnvs();
});

describe('matchLocale', () => {
	it('prefers an exact tag', () => {
		expect(matchLocale(['fr'], ['en', 'fr', 'pt-BR'], 'en')).toBe('fr');
	});

	it('drops the region when only the bare language ships', () => {
		expect(matchLocale(['en-GB'], ['en', 'fr'], 'en')).toBe('en');
	});

	it('accepts a regional catalogue for a request without a region', () => {
		expect(matchLocale(['pt'], ['en', 'pt-BR'], 'en')).toBe('pt-BR');
	});

	it('drops a script subtag as well as a region', () => {
		expect(matchLocale(['zh-Hant-TW'], ['en', 'zh'], 'en')).toBe('zh');
	});

	it('walks the whole preference list before giving up', () => {
		expect(matchLocale(['ca', 'es', 'en'], ['en', 'es'], 'en')).toBe('es');
	});

	it('prefers an exact match over a looser one earlier in the list', () => {
		expect(matchLocale(['pt-PT', 'en'], ['en', 'pt-BR'], 'en')).toBe('en');
	});

	it('follows the requested order, not the order the catalogues are declared in', () => {
		expect(matchLocale(['fr', 'en'], ['en', 'fr'], 'en')).toBe('fr');
	});

	it('compares tags case-insensitively', () => {
		expect(matchLocale(['EN-gb'], ['en'], 'en')).toBe('en');
	});

	it('falls back when nothing matches at all', () => {
		expect(matchLocale(['ja', 'ko'], ['en', 'fr'], 'en')).toBe('en');
	});

	it('falls back on an empty preference list', () => {
		expect(matchLocale([], ['en', 'fr'], 'en')).toBe('en');
	});
});

describe('systemLocales', () => {
	it('reads the ordered preference list', () => {
		speaking(['ca-ES', 'es-ES', 'en']);
		expect(systemLocales()).toEqual(['ca-ES', 'es-ES', 'en']);
	});

	it('falls back to the single language when the list is empty', () => {
		speaking([], 'fr-CA');
		expect(systemLocales()).toEqual(['fr-CA']);
	});

	it('reports nothing rather than throwing when neither is set', () => {
		speaking([], undefined);
		expect(systemLocales()).toEqual([]);
	});
});

describe('detectLocale', () => {
	it('answers with the source locale for a system that asks for it', () => {
		speaking(['en-GB', 'en']);
		expect(detectLocale()).toBe('en');
	});

	it('falls back to the source locale for a system Quiver has no catalogue for', () => {
		speaking(['fr-CA', 'fr']);
		expect(detectLocale()).toBe('en');
	});
});

describe('localeForcedByEnv', () => {
	it('is inert when the variable is unset', () => {
		expect(localeForcedByEnv()).toBeNull();
	});

	it('forces a shipped locale', () => {
		vi.stubEnv('VITE_QUIVER_LOCALE', 'en');
		expect(localeForcedByEnv()).toBe('en');
	});

	it('accepts the tag in any case', () => {
		vi.stubEnv('VITE_QUIVER_LOCALE', 'EN');
		expect(localeForcedByEnv()).toBe('en');
	});

	it('ignores a tag no catalogue exists for', () => {
		vi.stubEnv('VITE_QUIVER_LOCALE', 'klingon');
		expect(localeForcedByEnv()).toBeNull();
	});
});
