import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import { LOCALES, translateIn, type Locale } from './catalogue';
import { detectLocale, localeForcedByEnv } from './detect';
import type { MessageFor, MessageKey } from './locales/en';
import type { TranslateArgs } from './types';

export const LOCALE_STORAGE_KEY = 'quiver.locale';

export type LocalePreference = 'system' | Locale;

interface LocaleState {
	preference: LocalePreference;
	detected: Locale;
	setPreference: (preference: LocalePreference) => void;
	redetect: () => void;
}

export function normalisePreference(value: unknown): LocalePreference {
	if (value === 'system') return 'system';
	return LOCALES.find((tag) => tag === value) ?? 'system';
}

export const useLocaleStore = create<LocaleState>()(
	persist(
		(set) => ({
			preference: 'system',
			detected: detectLocale(),

			setPreference: (preference) => set({ preference }),
			redetect: () => set({ detected: detectLocale() }),
		}),
		{
			name: LOCALE_STORAGE_KEY,
			partialize: (s) => ({ preference: s.preference }),
			merge: (persisted, current) => ({
				...current,
				preference: normalisePreference((persisted as { preference?: unknown } | null)?.preference),
			}),
		}
	)
);

export function localeOf(state: Pick<LocaleState, 'preference' | 'detected'>): Locale {
	const forced = localeForcedByEnv();
	if (forced) return forced;
	return state.preference === 'system' ? state.detected : state.preference;
}

export function currentLocale(): Locale {
	return localeOf(useLocaleStore.getState());
}

export function t<K extends MessageKey>(key: K, ...args: TranslateArgs<MessageFor<K>>): string {
	return translateIn(currentLocale(), key, ...args);
}
