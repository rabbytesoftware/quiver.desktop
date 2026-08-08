import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import { LOCALES, translateIn, type Locale } from './catalogue';
import { detectLocale, localeForcedByEnv } from './detect';
import type { MessageFor, MessageKey } from './locales/en';
import type { TranslateArgs } from './types';

export const LOCALE_STORAGE_KEY = 'quiver.locale';

/**
 * `'system'` is the default and is a real value, not the absence of one: it
 * means "keep following the OS", so a user who moves their machine to another
 * language moves with it. Storing the resolved locale instead would silently
 * pin whatever language they happened to boot in the first time.
 */
export type LocalePreference = 'system' | Locale;

interface LocaleState {
	/** Persisted. What the user chose. */
	preference: LocalePreference;
	/**
	 * Ephemeral. What the system says, re-read on `languagechange`.
	 *
	 * Held in the store rather than computed on demand so that "follow the
	 * system" is REACTIVE: components subscribe to the store, and a `detectLocale()`
	 * call inside a selector would be re-run on every render and still never
	 * cause one.
	 */
	detected: Locale;
	setPreference: (preference: LocalePreference) => void;
	/** Re-reads `navigator.languages`. Wired to `languagechange` by `installLocaleSync`. */
	redetect: () => void;
}

/**
 * A saved preference is a string on disk, and the type above does not reach it.
 * A build that drops a locale — or a hand-edited `quiver.locale` — would
 * otherwise leave the app resolving to a catalogue that is not there.
 *
 * Falls back to `'system'` rather than to `SOURCE_LOCALE`, which is the more
 * useful wrong answer: someone whose German went away is better served by their
 * system language than by English, and if their system is German too they will
 * simply get English through the normal fallback and see nothing surprising.
 *
 * Same reasoning as `getScenario` in `@/lib/mock/world/scenarios`: a stale value
 * in localStorage must never be able to brick a release build, where the user
 * may not know the setting exists.
 */
export function normalisePreference(value: unknown): LocalePreference {
	if (value === 'system') return 'system';
	return LOCALES.find((tag) => tag === value) ?? 'system';
}

export const useLocaleStore = create<LocaleState>()(
	persist(
		(set) => ({
			preference: 'system',
			// Runs at import, which is before `ReactDOM.createRoot(...).render`.
			// That is the whole synchronous-first-render guarantee: by the time
			// any component asks for a string, the locale is already decided.
			detected: detectLocale(),

			setPreference: (preference) => set({ preference }),
			redetect: () => set({ detected: detectLocale() }),
		}),
		{
			name: LOCALE_STORAGE_KEY,
			// `detected` is a fact about the machine, not a setting: persisting it
			// would pin a stale answer across an OS language change.
			partialize: (s) => ({ preference: s.preference }),
			merge: (persisted, current) => ({
				...current,
				preference: normalisePreference((persisted as { preference?: unknown } | null)?.preference),
			}),
		}
	)
);

/**
 * The locale in force, given a preference and a detection.
 *
 * Pure and state-shaped rather than reading the store, so it can be used as a
 * zustand selector (`useLocaleStore(localeOf)`) and tested without one.
 *
 * The environment override wins over BOTH, matching `mockForcedByEnv`: a forced
 * locale that a saved preference could beat would make `VITE_QUIVER_LOCALE=de`
 * do nothing on the one machine that already had a preference saved — the
 * developer's.
 */
export function localeOf(state: Pick<LocaleState, 'preference' | 'detected'>): Locale {
	const forced = localeForcedByEnv();
	if (forced) return forced;
	return state.preference === 'system' ? state.detected : state.preference;
}

/** The locale in force right now, outside React. */
export function currentLocale(): Locale {
	return localeOf(useLocaleStore.getState());
}

/**
 * Translate outside a component — a store action, a listener, an error path.
 *
 * Reads the locale at call time rather than closing over one, so a string built
 * here is in the language in force when it is built, not when this module was
 * imported.
 *
 * Inside a component use `useTranslation()` instead: this one does not
 * subscribe, so a render that calls it will not re-run when the language
 * changes and the old string stays on screen.
 */
export function t<K extends MessageKey>(key: K, ...args: TranslateArgs<MessageFor<K>>): string {
	return translateIn(currentLocale(), key, ...args);
}
