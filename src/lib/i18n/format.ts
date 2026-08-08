// Everything localisable that is not a string: numbers, percentages, dates,
// times, and "3 days ago".
//
// All of it goes through `Intl`, which the webview already ships — every engine
// Quiver runs in (WKWebView, WebKitGTK, WebView2) has full ICU data built in,
// so this costs no bundle bytes and no dependency. The alternative, a
// formatting library, would ship a second copy of rules the OS already has and
// would then be wrong the year CLDR changes one of them.
//
// These take the locale as an argument rather than reading the store. Pure
// functions are what let a test pin a locale AND a time zone; `useTranslation`
// is what binds the active one for components.

/**
 * Formatter construction is the expensive part of `Intl` — it parses ICU
 * pattern data — and formatting is cheap. A settings panel that formats one
 * percentage per row would otherwise build a formatter per row per render.
 *
 * Keyed by locale AND options, because two call sites with different options
 * must not share. `JSON.stringify` is a sound key here for the same reason it
 * is usually unsound: these option objects are written as literals at the call
 * site, so their key order is stable per site. A different order would only
 * ever cost a duplicate cache entry, never a wrong answer.
 */
const cache = new Map<string, unknown>();

function memo<T>(kind: string, locale: string, options: object | undefined, build: () => T): T {
	const key = `${kind} ${locale} ${JSON.stringify(options ?? null)}`;
	const hit = cache.get(key);
	if (hit !== undefined) return hit as T;

	const built = build();
	cache.set(key, built);
	return built;
}

export function numberFormatter(locale: string, options?: Intl.NumberFormatOptions): Intl.NumberFormat {
	return memo('number', locale, options, () => new Intl.NumberFormat(locale, options));
}

export function dateFormatter(locale: string, options?: Intl.DateTimeFormatOptions): Intl.DateTimeFormat {
	return memo('date', locale, options, () => new Intl.DateTimeFormat(locale, options));
}

export function relativeTimeFormatter(
	locale: string,
	options?: Intl.RelativeTimeFormatOptions
): Intl.RelativeTimeFormat {
	return memo('relative', locale, options, () => new Intl.RelativeTimeFormat(locale, options));
}

/**
 * Cardinal plural rules. Cached alongside the formatters for the same reason
 * and at the same cost: `new Intl.PluralRules` is the expensive call, `.select`
 * is not, and `t` reaches for one on every plural message it renders.
 *
 * `type` defaults to `'cardinal'`, which is what a count needs. `'ordinal'` is
 * a different set of categories entirely ("1st", "2nd", "3rd") and would pick
 * the wrong form for a count — hence stated rather than left implicit.
 */
export function pluralRules(locale: string, options?: Intl.PluralRulesOptions): Intl.PluralRules {
	return memo('plural', locale, options, () => new Intl.PluralRules(locale, { type: 'cardinal', ...options }));
}

/**
 * A number in the reader's own digits and separators: `1234.5` is `1,234.5` in
 * English, `1.234,5` in German, and a narrow no-break space is the grouping
 * separator in French.
 *
 * The interpolator runs every numeric parameter through this, which is the
 * whole reason it exists — a template that reaches for a raw `${count}` renders
 * `1234` everywhere and reads as a bug to everyone but an anglophone.
 */
export function formatNumber(value: number, locale: string, options?: Intl.NumberFormatOptions): string {
	return numberFormatter(locale, options).format(value);
}

/**
 * TAKES A FRACTION, NOT A PERCENTAGE. `formatPercent(0.4)` is `40%`;
 * `formatPercent(40)` is `4,000%`, and it will not warn you.
 *
 * That is `Intl`'s convention rather than a choice made here, and it is the one
 * mistake this function invites, so it is named in the signature: a caller
 * holding a 0–100 value (the chaos sliders do) divides at the call site.
 *
 * Worth going through `Intl` for what looks like `${n}%`: the symbol is not
 * always a trailing `%` — several European locales put a space before it,
 * Turkish puts the symbol first, and Arabic has a character of its own.
 */
export function formatPercent(fraction: number, locale: string, options?: Intl.NumberFormatOptions): string {
	return numberFormatter(locale, { style: 'percent', ...options }).format(fraction);
}

/**
 * A date, without a time.
 *
 * TIME ZONE: with no `timeZone` option `Intl` uses the host's, which is right
 * for the app and a trap for tests — an ISO instant near midnight UTC renders
 * as the previous day for anyone west of Greenwich, so a test asserting a
 * literal date MUST pin `timeZone` or it passes in CI and fails in Denver.
 */
export function formatDate(
	value: Date | string | number,
	locale: string,
	options?: Intl.DateTimeFormatOptions
): string {
	return dateFormatter(locale, { dateStyle: 'medium', ...options }).format(toDate(value));
}

/** A date and a time together. Same time-zone caveat as `formatDate`. */
export function formatDateTime(
	value: Date | string | number,
	locale: string,
	options?: Intl.DateTimeFormatOptions
): string {
	return dateFormatter(locale, { dateStyle: 'medium', timeStyle: 'short', ...options }).format(toDate(value));
}

/** Largest first: the first threshold a difference clears is the unit it is said in. */
const RELATIVE_UNITS: ReadonlyArray<[Intl.RelativeTimeFormatUnit, number]> = [
	['year', 365 * 24 * 60 * 60 * 1000],
	['month', 30 * 24 * 60 * 60 * 1000],
	['week', 7 * 24 * 60 * 60 * 1000],
	['day', 24 * 60 * 60 * 1000],
	['hour', 60 * 60 * 1000],
	['minute', 60 * 1000],
	['second', 1000],
];

/**
 * "3 days ago", "in 2 hours" — and, because `numeric: 'auto'` is the default
 * here, "yesterday" and "tomorrow" wherever the language has a word for it.
 *
 * The month and year thresholds are approximations (30 and 365 days), which is
 * fine for the only thing this is for: an age shown next to an installed arrow.
 * It is not a calendar difference and must not be used as one.
 *
 * `now` is a parameter rather than a `Date.now()` call inside, so a test states
 * the reference instant instead of racing the clock.
 */
export function formatRelativeTime(
	value: Date | string | number,
	locale: string,
	now: Date | number = Date.now(),
	options?: Intl.RelativeTimeFormatOptions
): string {
	const delta = toDate(value).getTime() - (now instanceof Date ? now.getTime() : now);
	const formatter = relativeTimeFormatter(locale, { numeric: 'auto', ...options });

	for (const [unit, ms] of RELATIVE_UNITS) {
		// Compared on the absolute value but formatted with the sign intact: a
		// past instant has a negative delta, and that sign is what `Intl` turns
		// into "ago" rather than "in".
		if (Math.abs(delta) >= ms) return formatter.format(Math.trunc(delta / ms), unit);
	}

	// Under a second in either direction. `numeric: 'auto'` renders this as
	// "now" rather than "in 0 seconds".
	return formatter.format(0, 'second');
}

function toDate(value: Date | string | number): Date {
	return value instanceof Date ? value : new Date(value);
}

/** Test seam: the memo is module state, and would otherwise outlive the case that filled it. */
export function clearFormatterCache(): void {
	cache.clear();
}
