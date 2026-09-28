/**
 * Wire helpers for `GET /v0/config`.
 *
 * A section is not flat: `arrows.auto_retry` and `search.unmarked` are objects.
 * Anything that lists a section's settings goes through here, so a nested
 * object becomes dotted rows (`unmarked.min_stars`) instead of `[object Object]`.
 */

export interface ConfigRow {
	/** Dotted wire path relative to the section, e.g. `unmarked.min_stars`. */
	key: string;
	value: unknown;
}

export interface ConfigDisplayRow {
	key: string;
	value: string;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * Walks a config object down to its leaves. An empty object is kept as a leaf,
 * so a group with nothing in it still shows up rather than vanishing.
 */
export function flattenConfig(section: Record<string, unknown>, prefix = ''): ConfigRow[] {
	return Object.entries(section).flatMap(([name, value]) => {
		const key = prefix ? `${prefix}.${name}` : name;
		if (isPlainObject(value) && Object.keys(value).length > 0) return flattenConfig(value, key);
		return [{ key, value }];
	});
}

/** Renders one leaf the way the settings list shows it: scalars as-is, arrays comma-joined. */
export function formatConfigValue(value: unknown): string {
	if (Array.isArray(value)) {
		if (value.length === 0) return '[]';
		return value
			.map((item) => (typeof item === 'object' && item !== null ? JSON.stringify(item) : String(item)))
			.join(', ');
	}
	if (isPlainObject(value)) return '{}';
	return String(value);
}

export function toConfigDisplayRows(section: Record<string, unknown>): ConfigDisplayRow[] {
	return flattenConfig(section).map(({ key, value }) => ({ key, value: formatConfigValue(value) }));
}
