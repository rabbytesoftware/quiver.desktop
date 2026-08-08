// The public surface. Feature code imports from `@/lib/i18n` and nothing else
// in here, so the split between catalogue, detection, storage and formatting
// stays an implementation detail that can move.
//
//     import { useTranslation } from '@/lib/i18n';
//
//     const { t, formatPercent } = useTranslation();
//     t('settings.title')                          // "Settings"
//     t('settings.version.text', { version })      // "Quiver 0.1.0"
//     t('settings.version.remaining', { count })   // "1 more tap…" / "3 more taps…"
//     formatPercent(0.4)                           // "40%"
//
// A key that does not exist, a `{placeholder}` left without a value, and a
// plural key called without `count` are all compile errors.

export { installLocaleSync } from './bootstrap';
export {
	CATALOGUES,
	createTranslator,
	interpolate,
	LOCALES,
	renderMessage,
	selectPluralForm,
	SOURCE_LOCALE,
	translateIn,
	type Locale,
	type Translator,
} from './catalogue';
export { detectLocale, localeForcedByEnv, matchLocale, systemLocales } from './detect';
export {
	clearFormatterCache,
	formatDate,
	formatDateTime,
	formatNumber,
	formatPercent,
	formatRelativeTime,
} from './format';
export type { LocaleCatalogue, MessageKey } from './locales/en';
export { useLocale, useTranslation } from './react';
export {
	currentLocale,
	LOCALE_STORAGE_KEY,
	localeOf,
	normalisePreference,
	t,
	useLocaleStore,
	type LocalePreference,
} from './store';
export type { Catalogue, Message, MessageParams, PluralMessage } from './types';
