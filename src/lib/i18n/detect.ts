import { LOCALES, SOURCE_LOCALE, type Locale } from './catalogue';

// Which language this webview should speak, decided before anything renders.
//
// WHY `navigator.languages`, and not something better-behaved — the same
// argument `src/lib/platform.ts` makes for reading the user-agent string
// instead of `@tauri-apps/plugin-os`, and for the same reason it is worth
// making twice:
//
//  - `@tauri-apps/plugin-os` has a `locale()` that answers properly. It is
//    `async`. Every string in the app then depends on a promise, which means
//    either a Suspense boundary around the entire UI or a first paint in the
//    wrong language followed by a re-render into the right one. A flash of
//    untranslated content is the exact defect this module exists to prevent,
//    and it is worse than the title-bar flicker `platform.ts` was avoiding —
//    it is every label on the screen, not one strip of chrome;
//  - the same objection kills reading it over IPC from the Rust side, and adds
//    a round trip;
//  - `Intl.DateTimeFormat().resolvedOptions().locale` IS synchronous, but it
//    answers with the formatting locale the engine settled on, not the ordered
//    preference list the user actually set. On a machine set to "display in
//    Catalan, format numbers as Spanish" it names the wrong one.
//
// `navigator.languages` is synchronous, populated before first paint, and is
// the ordered list itself — "Catalan, then Spanish, then English" — which is
// what makes a sensible answer possible for a user whose first choice Quiver
// does not ship. Every webview Quiver runs in implements it; `navigator.language`
// is the fallback for the one that somehow does not.
//
// It has one real weakness, and it is the reverse of the user-agent trap: a
// webview may report the browser's UI language rather than the OS's. In a Tauri
// app those are the same thing — the webview has no language setting of its
// own — and the persisted preference in `./store.ts` is the escape hatch for
// anyone the guess is wrong for.

/**
 * The user's ordered language preferences, most-wanted first, as BCP-47 tags.
 *
 * Defensive about a missing or empty `languages` rather than trusting the spec:
 * this is read at module scope during boot, and an empty array here would send
 * `matchLocale` straight to the fallback for a user who did state a preference.
 */
export function systemLocales(): readonly string[] {
	const listed = navigator.languages;
	if (listed && listed.length > 0) return listed;
	return navigator.language ? [navigator.language] : [];
}

/**
 * The best of `supported` for someone who asked for `requested`, in order.
 *
 * Matching is two-sided and case-insensitive, because BCP-47 tags are compared
 * far more loosely than they are written:
 *
 *   requested `en-GB`, supported `en`     → `en`     (drop the region)
 *   requested `pt`,    supported `pt-BR`  → `pt-BR`  (the only Portuguese there is)
 *   requested `zh-Hant-TW`, supported `zh` → `zh`    (drop everything after the language)
 *
 * The whole list is walked before falling back, and in the requester's order —
 * so "Catalan, Spanish, English" against a Spanish-and-English build gets
 * Spanish, not English. Taking the first entry only, which is the shortcut this
 * invites, gets that user the wrong language whenever their first choice is
 * missing.
 *
 * A tag whose language subtag matches is preferred over nothing but never over
 * an exact hit: the exact pass runs to completion first.
 *
 * Generic over the supported set so the behaviour can be tested against a
 * realistic multi-locale list without the app having to ship one.
 */
export function matchLocale<T extends string>(requested: readonly string[], supported: readonly T[], fallback: T): T {
	// Both passes iterate REQUESTED, not SUPPORTED, and that is the difference
	// between honouring a preference and honouring a build. Searching the
	// supported list on the outside returns whichever catalogue happens to be
	// declared first — English, in every registry ever written — to a user who
	// asked for French and listed English as their third choice.
	for (const want of requested) {
		const exact = supported.find((tag) => sameTag(want, tag));
		if (exact) return exact;
	}

	// Only once no exact tag matched anywhere in the list: a loose hit on the
	// first preference must not beat an exact hit on the second.
	for (const want of requested) {
		const byLanguage = supported.find((tag) => sameTag(language(want), language(tag)));
		if (byLanguage) return byLanguage;
	}

	return fallback;
}

function sameTag(a: string, b: string): boolean {
	return a.toLowerCase() === b.toLowerCase();
}

/** `zh-Hant-TW` → `zh`. The subtag before the first hyphen is always the language. */
function language(tag: string): string {
	return tag.split('-')[0];
}

/**
 * Force a locale for one run, regardless of the system and of the saved
 * preference — `VITE_QUIVER_LOCALE=de bun run dev`.
 *
 * Mirrors `mockForcedByEnv` in `@/lib/mock/preference` deliberately, down to
 * the behaviour the Settings row is then obliged to have: it FORCES rather than
 * seeds, so the picker is disabled and says why. A seeded value would let the
 * picker appear to work, reload, and come straight back to the forced language.
 *
 * An unshipped or misspelled tag is ignored rather than honoured-and-empty:
 * `VITE_QUIVER_LOCALE=klingon` leaves the app exactly as it was instead of
 * rendering a catalogue that does not exist.
 */
export function localeForcedByEnv(): Locale | null {
	const requested = import.meta.env.VITE_QUIVER_LOCALE;
	if (!requested) return null;

	const match = LOCALES.find((tag) => tag.toLowerCase() === requested.toLowerCase());
	return match ?? null;
}

/**
 * The locale to speak when the saved preference is "follow the system".
 *
 * Synchronous, total, and safe to call at module scope — which `./store.ts`
 * does, to seed its initial state before React mounts.
 */
export function detectLocale(): Locale {
	return matchLocale(systemLocales(), LOCALES, SOURCE_LOCALE);
}
