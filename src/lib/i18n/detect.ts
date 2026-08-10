import { LOCALES, SOURCE_LOCALE, type Locale } from './catalogue';

export function systemLocales(): readonly string[] {
	const listed = navigator.languages;
	if (listed && listed.length > 0) return listed;
	return navigator.language ? [navigator.language] : [];
}

export function matchLocale<T extends string>(requested: readonly string[], supported: readonly T[], fallback: T): T {
	for (const want of requested) {
		const exact = supported.find((tag) => sameTag(want, tag));
		if (exact) return exact;
	}

	for (const want of requested) {
		const byLanguage = supported.find((tag) => sameTag(language(want), language(tag)));
		if (byLanguage) return byLanguage;
	}

	return fallback;
}

function sameTag(a: string, b: string): boolean {
	return a.toLowerCase() === b.toLowerCase();
}

function language(tag: string): string {
	return tag.split('-')[0];
}

export function localeForcedByEnv(): Locale | null {
	const requested = import.meta.env.VITE_QUIVER_LOCALE;
	if (!requested) return null;

	const match = LOCALES.find((tag) => tag.toLowerCase() === requested.toLowerCase());
	return match ?? null;
}

export function detectLocale(): Locale {
	return matchLocale(systemLocales(), LOCALES, SOURCE_LOCALE);
}
