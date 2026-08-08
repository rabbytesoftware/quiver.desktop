import { useMemo } from 'react';

import { createTranslator, type Locale, type Translator } from './catalogue';
import { localeOf, useLocaleStore } from './store';

/** The locale in force, as a subscription: a component using this re-renders when it changes. */
export function useLocale(): Locale {
	return useLocaleStore(localeOf);
}

/**
 * Everything localisable, bound to the active locale.
 *
 *     const { t } = useTranslation();
 *     <DialogTitle>{t('settings.title')}</DialogTitle>
 *
 * MEMOISED ON THE LOCALE, which is not a micro-optimisation. `createTranslator`
 * builds a fresh object with fresh closures; returned unmemoised, `t` would
 * have a new identity on every render, and every `useMemo`, `useCallback` and
 * `React.memo` downstream that lists it as a dependency would be defeated. The
 * cost lands nowhere near i18n and nobody would connect the two.
 *
 * The selector returns a string, so a locale that has not changed produces no
 * render at all — zustand compares the selected value, not the state object.
 */
export function useTranslation(): Translator {
	const locale = useLocale();
	return useMemo(() => createTranslator(locale), [locale]);
}
