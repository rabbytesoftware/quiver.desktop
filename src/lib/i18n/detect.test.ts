import { afterEach, describe, expect, it, vi } from 'vitest';

import { restoreLanguages, speaking } from '@/__mocks__/navigator-language';

import { detectLocale, localeForcedByEnv, matchLocale, systemLocales } from './detect';

afterEach(() => {
	restoreLanguages();
	vi.unstubAllEnvs();
});

// The supported set is stated per case rather than taken from `LOCALES`. Quiver
// ships one catalogue today, and a matcher tested only against a one-element
// list is a matcher that has not been tested — every interesting rule here
// (region stripping, ordering, both-direction prefixes) needs at least two.
describe('matchLocale', () => {
	it('prefers an exact tag', () => {
		expect(matchLocale(['fr'], ['en', 'fr', 'pt-BR'], 'en')).toBe('fr');
	});

	it('drops the region when only the bare language ships', () => {
		expect(matchLocale(['en-GB'], ['en', 'fr'], 'en')).toBe('en');
	});

	// The other direction, which the obvious implementation misses: someone
	// asking for Portuguese in general should get the only Portuguese there is.
	it('accepts a regional catalogue for a request without a region', () => {
		expect(matchLocale(['pt'], ['en', 'pt-BR'], 'en')).toBe('pt-BR');
	});

	it('drops a script subtag as well as a region', () => {
		expect(matchLocale(['zh-Hant-TW'], ['en', 'zh'], 'en')).toBe('zh');
	});

	// The whole list, in the user's order. Taking `requested[0]` only — the
	// shortcut this invites — hands English to a Catalan speaker who explicitly
	// listed Spanish second.
	it('walks the whole preference list before giving up', () => {
		expect(matchLocale(['ca', 'es', 'en'], ['en', 'es'], 'en')).toBe('es');
	});

	// An exact match further down the list must still beat a language-only match
	// higher up: the exact pass runs to completion before the loose one starts.
	it('prefers an exact match over a looser one earlier in the list', () => {
		expect(matchLocale(['pt-PT', 'en'], ['en', 'pt-BR'], 'en')).toBe('en');
	});

	// The user's order decides, not the registry's. Iterating the supported list
	// on the outside — the natural way to write this — hands English to someone
	// who asked for French first, because English is declared first in every
	// registry anyone writes.
	it('follows the requested order, not the order the catalogues are declared in', () => {
		expect(matchLocale(['fr', 'en'], ['en', 'fr'], 'en')).toBe('fr');
	});

	// BCP-47 tags are case-insensitive by spec and inconsistently cased in the
	// wild — `zh-Hant`, `en-US`, `en-us` are all the same request.
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

	// A webview that populates `language` and leaves `languages` empty would
	// otherwise send a user who did state a preference straight to the fallback.
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

	// Quiver ships one catalogue, so this is the fallback path — and it is the
	// path most users of an un-translated build take, so it is the one that must
	// not throw or return `undefined`.
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

	// Honouring an unshipped tag would resolve to a catalogue that is not there.
	// Ignoring it leaves the app exactly as it was, which is the only harmless
	// answer to a typo in an environment variable.
	it('ignores a tag no catalogue exists for', () => {
		vi.stubEnv('VITE_QUIVER_LOCALE', 'klingon');
		expect(localeForcedByEnv()).toBeNull();
	});
});
