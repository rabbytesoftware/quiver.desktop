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

export function pluralRules(locale: string, options?: Intl.PluralRulesOptions): Intl.PluralRules {
	return memo('plural', locale, options, () => new Intl.PluralRules(locale, { type: 'cardinal', ...options }));
}

export function formatNumber(value: number, locale: string, options?: Intl.NumberFormatOptions): string {
	return numberFormatter(locale, options).format(value);
}

export function formatPercent(fraction: number, locale: string, options?: Intl.NumberFormatOptions): string {
	return numberFormatter(locale, { style: 'percent', ...options }).format(fraction);
}

export function formatDate(
	value: Date | string | number,
	locale: string,
	options?: Intl.DateTimeFormatOptions
): string {
	return dateFormatter(locale, { dateStyle: 'medium', ...options }).format(toDate(value));
}

export function formatDateTime(
	value: Date | string | number,
	locale: string,
	options?: Intl.DateTimeFormatOptions
): string {
	return dateFormatter(locale, { dateStyle: 'medium', timeStyle: 'short', ...options }).format(toDate(value));
}

const RELATIVE_UNITS: ReadonlyArray<[Intl.RelativeTimeFormatUnit, number]> = [
	['year', 365 * 24 * 60 * 60 * 1000],
	['month', 30 * 24 * 60 * 60 * 1000],
	['week', 7 * 24 * 60 * 60 * 1000],
	['day', 24 * 60 * 60 * 1000],
	['hour', 60 * 60 * 1000],
	['minute', 60 * 1000],
	['second', 1000],
];

export function formatRelativeTime(
	value: Date | string | number,
	locale: string,
	now: Date | number = Date.now(),
	options?: Intl.RelativeTimeFormatOptions
): string {
	const delta = toDate(value).getTime() - (now instanceof Date ? now.getTime() : now);
	const formatter = relativeTimeFormatter(locale, { numeric: 'auto', ...options });

	for (const [unit, ms] of RELATIVE_UNITS) {
		if (Math.abs(delta) >= ms) return formatter.format(Math.trunc(delta / ms), unit);
	}

	return formatter.format(0, 'second');
}

function toDate(value: Date | string | number): Date {
	return value instanceof Date ? value : new Date(value);
}

export function clearFormatterCache(): void {
	cache.clear();
}
