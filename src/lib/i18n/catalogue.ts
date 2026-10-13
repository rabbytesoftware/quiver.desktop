import { formatDate, formatDateTime, formatNumber, formatPercent, formatRelativeTime, pluralRules } from './format';
import { en, type LocaleCatalogue, type MessageFor, type MessageKey } from './locales/en';
import type { Message, MessageParams, PluralMessage, TranslateArgs } from './types';

export const CATALOGUES = { en } as const satisfies Readonly<Record<string, LocaleCatalogue>>;

export type Locale = keyof typeof CATALOGUES;

export const SOURCE_LOCALE: Locale = 'en';

export const LOCALES = Object.keys(CATALOGUES) as readonly Locale[];

const PLACEHOLDER = /\{(\w+)\}/g;

export function interpolate(template: string, locale: string, params?: MessageParams): string {
	if (!params) return template;

	return template.replace(PLACEHOLDER, (whole, name: string) => {
		const value = params[name];
		if (value === undefined) return whole;
		return typeof value === 'number' ? formatNumber(value, locale) : value;
	});
}

export function selectPluralForm(message: PluralMessage, locale: string, count: number): string {
	return message[pluralRules(locale).select(count)] ?? message.other;
}

export function renderMessage(message: Message, locale: string, params?: MessageParams): string {
	if (typeof message === 'string') return interpolate(message, locale, params);

	const count = typeof params?.count === 'number' ? params.count : 0;
	return interpolate(selectPluralForm(message, locale, count), locale, { ...params, count });
}

function messageFor(locale: Locale, key: MessageKey): Message {
	const message: Message | undefined = CATALOGUES[locale][key] ?? CATALOGUES[SOURCE_LOCALE][key];
	return message ?? key;
}

export function translateIn<K extends MessageKey>(
	locale: Locale,
	key: K,
	...args: TranslateArgs<MessageFor<K>>
): string {
	const [params] = args as unknown as [MessageParams | undefined];
	return renderMessage(messageFor(locale, key), locale, params);
}

export interface Translator {
	readonly locale: Locale;
	t<K extends MessageKey>(key: K, ...args: TranslateArgs<MessageFor<K>>): string;
	formatNumber(value: number, options?: Intl.NumberFormatOptions): string;
	formatPercent(fraction: number, options?: Intl.NumberFormatOptions): string;
	formatDate(value: Date | string | number, options?: Intl.DateTimeFormatOptions): string;
	formatDateTime(value: Date | string | number, options?: Intl.DateTimeFormatOptions): string;
	formatRelativeTime(
		value: Date | string | number,
		now?: Date | number,
		options?: Intl.RelativeTimeFormatOptions
	): string;
}

export function createTranslator(locale: Locale): Translator {
	return {
		locale,
		t: (key, ...args) => translateIn(locale, key, ...args),
		formatNumber: (value, options) => formatNumber(value, locale, options),
		formatPercent: (fraction, options) => formatPercent(fraction, locale, options),
		formatDate: (value, options) => formatDate(value, locale, options),
		formatDateTime: (value, options) => formatDateTime(value, locale, options),
		formatRelativeTime: (value, now, options) => formatRelativeTime(value, locale, now, options),
	};
}
