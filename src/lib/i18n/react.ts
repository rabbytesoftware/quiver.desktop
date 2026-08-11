import { useMemo } from 'react';

import { createTranslator, type Locale, type Translator } from './catalogue';
import { localeOf, useLocaleStore } from './store';

export function useLocale(): Locale {
	return useLocaleStore(localeOf);
}

export function useTranslation(): Translator {
	const locale = useLocale();
	return useMemo(() => createTranslator(locale), [locale]);
}
