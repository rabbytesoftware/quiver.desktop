import { formatDate, formatDateTime, formatNumber, formatPercent, formatRelativeTime, pluralRules } from './format';
import { en, type LocaleCatalogue, type MessageFor, type MessageKey } from './locales/en';
import type { Message, MessageParams, PluralMessage, TranslateArgs } from './types';

/**
 * Every catalogue the app ships, and the single place a new locale is
 * registered. `Locale`, `LOCALES` and the Settings picker are all derived from
 * this object, so adding one is an import and a line — never a second list to
 * keep in step.
 *
 * ONLY `en` TODAY, and deliberately so. A half-finished second locale is worse
 * than none: `LocaleCatalogue` would force every key to exist, so the only way
 * to add a language without translating all of it is to paste English under a
 * foreign tag — which ships a picker offering a language the app does not
 * actually speak. The structure is what this change is for; the translations
 * are a separate, human piece of work.
 */
export const CATALOGUES = { en } as const satisfies Readonly<Record<string, LocaleCatalogue>>;

/** A locale Quiver has a catalogue for. Not a BCP-47 tag in general — those arrive from the system and are matched onto this. */
export type Locale = keyof typeof CATALOGUES;

/** The locale every message is written in, and the fallback for everything. */
export const SOURCE_LOCALE: Locale = 'en';

/**
 * Cast because `Object.keys` is typed `string[]` — soundly, since a JS object
 * can carry keys its type does not mention. This one cannot: it is a literal
 * declared three lines up, frozen by `as const`, and never written to.
 */
export const LOCALES = Object.keys(CATALOGUES) as readonly Locale[];

/**
 * `{name}` — chosen over ICU MessageFormat's `{name}` plus its select/plural
 * sublanguage because the plural half is handled by `Intl.PluralRules` against
 * a keyed variant instead, which needs no parser. Word characters only, so a
 * literal brace in copy (JSON in a code sample, a CSS snippet) is left alone
 * rather than eaten as a malformed placeholder.
 */
const PLACEHOLDER = /\{(\w+)\}/g;

/**
 * Fill the `{holes}` in a template.
 *
 * ONE PASS, and that matters: `String.replace` never rescans what a replacement
 * produced, so a value that itself contains `{count}` — a host name, a search
 * query someone typed — is inserted verbatim instead of being re-expanded.
 * Looping until the string stops changing, the obvious alternative, is a
 * user-controlled infinite loop.
 *
 * NUMBERS ARE FORMATTED, strings are not. That is the point of routing
 * interpolation through here rather than through a template literal: `1234`
 * has to become `1,234` or `1.234` depending on who is reading, and a call site
 * cannot be trusted to remember. Pass a string when the digits are an
 * identifier rather than a quantity — a version, a port, an error code — since
 * grouping separators in `1.0.0` or `8080` would be actively wrong.
 *
 * A missing value leaves the placeholder standing rather than printing
 * `undefined`: the types make it unreachable from TypeScript, and if it is ever
 * reached anyway, `{name}` in the UI names the bug.
 */
export function interpolate(template: string, locale: string, params?: MessageParams): string {
	if (!params) return template;

	return template.replace(PLACEHOLDER, (whole, name: string) => {
		const value = params[name];
		if (value === undefined) return whole;
		return typeof value === 'number' ? formatNumber(value, locale) : value;
	});
}

/**
 * Which grammatical form `count` takes in `locale`.
 *
 * The reason this is not a ternary at the call site: English has two forms and
 * almost every call site written by an English speaker encodes exactly that
 * assumption. Russian has four (1 файл, 2 файла, 5 файлов, 1.5 файла), Welsh
 * and Arabic have six, Japanese has one. `Intl.PluralRules` knows all of them
 * and the app does not have to.
 *
 * Falls back to `other` for a form the catalogue omits. That is a translation
 * gap — Russian text under `other` for a count of 2 is grammatically wrong —
 * but it renders a real sentence, and a locale is far likelier to be missing a
 * rare form than to want a crash. `PluralMessage` requires `other` for exactly
 * this reason: it is the one form that is always there to fall back to.
 */
export function selectPluralForm(message: PluralMessage, locale: string, count: number): string {
	return message[pluralRules(locale).select(count)] ?? message.other;
}

/**
 * A message plus its parameters, rendered. Pure and locale-explicit, so the
 * plural and interpolation rules can be tested against locales the app does not
 * ship a catalogue for.
 *
 * A plural message reached without a numeric `count` is treated as zero rather
 * than as `NaN`: `Intl.PluralRules.select(NaN)` answers `other` in most locales
 * and throws in none, so the failure would be an invisibly wrong sentence. The
 * type of `t` already requires `count` on a plural key; this is what the one
 * call that bypassed the types does instead.
 */
export function renderMessage(message: Message, locale: string, params?: MessageParams): string {
	if (typeof message === 'string') return interpolate(message, locale, params);

	// The defaulted count is put BACK into the bag before interpolating, so the
	// number that chose the form is the number that appears in it. Passing the
	// original `params` through instead picks the `zero` form and then leaves a
	// literal `{count}` sitting inside it — two different answers to the same
	// missing value, in one sentence.
	const count = typeof params?.count === 'number' ? params.count : 0;
	return interpolate(selectPluralForm(message, locale, count), locale, { ...params, count });
}

/**
 * Three layers of fallback for something the type system already makes
 * impossible, and each covers a case the one above it cannot:
 *
 *  - the active locale's catalogue is the answer;
 *  - English is the answer when a locale was registered in `CATALOGUES` with a
 *    hole in it. `LocaleCatalogue` forbids that and locales.test.ts asserts it
 *    at runtime, but neither survives a hand-patched bundle;
 *  - the key itself is the answer when even English has no such message, which
 *    is what a `key as MessageKey` cast at a call site can produce.
 *
 * The point of all three is that a gap renders SOMETHING legible. `undefined`
 * in a button is a bug report with no information in it; the key is a bug
 * report that names the missing message.
 */
function messageFor(locale: Locale, key: MessageKey): Message {
	const message: Message | undefined = CATALOGUES[locale][key] ?? CATALOGUES[SOURCE_LOCALE][key];
	return message ?? key;
}

/**
 * Translate `key` into `locale`.
 *
 * The generic is what carries the type safety: `K` is inferred as the literal
 * key, `MessageFor<K>` is the literal message behind it, and `TranslateArgs`
 * turns that into either no second argument or exactly the parameters the
 * message interpolates. A misspelled key, a missing `{name}`, or a `count`
 * left off a plural are all `tsc` errors.
 */
export function translateIn<K extends MessageKey>(
	locale: Locale,
	key: K,
	...args: TranslateArgs<MessageFor<K>>
): string {
	// `TranslateArgs<MessageFor<K>>` is unresolved while `K` is still generic,
	// so TypeScript will not index it. Nothing downstream needs that precision —
	// the renderer takes an optional bag either way — so it is discarded once,
	// here, rather than weakening the signature callers see.
	const [params] = args as unknown as [MessageParams | undefined];
	return renderMessage(messageFor(locale, key), locale, params);
}

/** Everything localisable, with one locale already bound. See `useTranslation`. */
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

/**
 * Bind a locale to every formatter at once.
 *
 * Built per locale rather than per call so that `useTranslation` can memoise it
 * on the locale alone: a component receiving a new `t` identity on every render
 * defeats every `useMemo`, `useCallback` and `React.memo` downstream of it, and
 * that is exactly the kind of regression nobody attributes to i18n.
 */
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
