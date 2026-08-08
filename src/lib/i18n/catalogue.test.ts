import { describe, expect, it } from 'vitest';

import { createTranslator, interpolate, renderMessage, selectPluralForm, translateIn } from './catalogue';
import type { PluralMessage } from './types';

// The plural machinery is tested against locales Quiver ships no catalogue for.
// That is the point: `Intl.PluralRules` supplies the grammar, so the rules can
// be — and have to be — proven correct before a translator ever arrives. Waiting
// for a Russian catalogue to exist before checking that `many` is reachable is
// how a catalogue arrives and silently renders the wrong noun.

/** Russian: four cardinal categories, and every one of them reachable. */
const RU_ARROWS: PluralMessage = {
	one: '{count} стрела',
	few: '{count} стрелы',
	many: '{count} стрел',
	other: '{count} стрелы',
};

/** Welsh: all six CLDR categories, which is as many as any language has. */
const CY_CATS: PluralMessage = {
	zero: '{count} cathod',
	one: '{count} gath',
	two: '{count} gath',
	few: '{count} cath',
	many: '{count} chath',
	other: '{count} cath',
};

describe('key resolution', () => {
	it('answers with the message behind the key', () => {
		expect(translateIn('en', 'settings.title')).toBe('Settings');
	});

	// Unreachable from TypeScript — the cast is what a `key as MessageKey` at a
	// call site would do. The key itself is a bug report that names the missing
	// message; `undefined` in a button is one that names nothing.
	it('renders the key itself rather than `undefined` when no message exists', () => {
		// Cast to a specific parameterless key rather than to `MessageKey`: the
		// argument list is derived FROM the key, so the full union asks for the
		// parameters of every message in the catalogue at once.
		const absent = 'settings.nope.not.a.key' as 'settings.title';
		expect(translateIn('en', absent)).toBe('settings.nope.not.a.key');
	});
});

describe('interpolation', () => {
	it('fills a named hole', () => {
		expect(translateIn('en', 'settings.version.text', { version: '0.1.0' })).toBe('Quiver 0.1.0');
	});

	it('leaves a template with no parameters alone', () => {
		expect(interpolate('nothing to fill', 'en')).toBe('nothing to fill');
	});

	// A raw `${count}` in a template renders `1234` for everyone. This is the
	// reason interpolation is a function rather than a template literal.
	it('formats a numeric value for the locale, not for the developer', () => {
		expect(interpolate('{n} arrows', 'en', { n: 1234 })).toBe('1,234 arrows');
		expect(interpolate('{n} Pfeile', 'de-DE', { n: 1234 })).toBe('1.234 Pfeile');
	});

	// The classic re-entrancy bug in a hand-rolled interpolator: loop until the
	// string stops changing, and a value someone typed becomes a template.
	it('does not rescan what it just substituted', () => {
		expect(interpolate('{name} is here', 'en', { name: '{name} {count}' })).toBe('{name} {count} is here');
	});

	it('leaves an unfilled hole standing, so the gap is visible', () => {
		expect(interpolate('{a} and {b}', 'en', { a: 'this' })).toBe('this and {b}');
	});

	// Braces in copy — a JSON snippet, a CSS rule — must survive. `\w+` is what
	// keeps `{ "id": 1 }` from being read as a malformed placeholder.
	it('ignores braces that are not a placeholder', () => {
		expect(interpolate('use { "id": 1 } here', 'en', { id: 'x' })).toBe('use { "id": 1 } here');
	});
});

describe('pluralisation', () => {
	it('picks the English form on a two-category language', () => {
		expect(translateIn('en', 'settings.version.remaining', { count: 1 })).toBe('1 more tap…');
		expect(translateIn('en', 'settings.version.remaining', { count: 3 })).toBe('3 more taps…');
		// Zero is `other` in English, and this is where a naive `count === 1`
		// ternary happens to agree — which is why it is not the interesting case.
		expect(translateIn('en', 'settings.version.remaining', { count: 0 })).toBe('0 more taps…');
	});

	// FOUR categories, and the boundaries are not where an English speaker would
	// guess: 21 is `one`, 11 is `many`, 2 is `few`. No ternary written by hand
	// gets this right, which is the whole argument for `Intl.PluralRules`.
	it('picks all four Russian forms, including the ones English has no analogue for', () => {
		const ru = (count: number) => renderMessage(RU_ARROWS, 'ru', { count });

		expect(ru(1)).toBe('1 стрела');
		expect(ru(2)).toBe('2 стрелы');
		expect(ru(5)).toBe('5 стрел');
		expect(ru(11)).toBe('11 стрел');
		expect(ru(21)).toBe('21 стрела');
	});

	it('picks all six Welsh forms', () => {
		const cy = (count: number) => renderMessage(CY_CATS, 'cy', { count });

		expect(cy(0)).toBe('0 cathod');
		expect(cy(1)).toBe('1 gath');
		expect(cy(2)).toBe('2 gath');
		expect(cy(3)).toBe('3 cath');
		expect(cy(6)).toBe('6 chath');
		expect(cy(10)).toBe('10 cath');
	});

	// A translator who fills in only the forms they were shown must not produce
	// a crash or a blank. Wrong grammar is recoverable; a missing sentence is not.
	it('falls back to `other` for a form the catalogue omits', () => {
		const partial: PluralMessage = { one: 'один', other: 'много' };
		expect(selectPluralForm(partial, 'ru', 1)).toBe('один');
		expect(selectPluralForm(partial, 'ru', 5)).toBe('много');
	});

	// Unreachable through `t`, whose type demands `count` on a plural key. This
	// is what the one call that got past the types does instead of selecting on
	// NaN and landing on a silently wrong form.
	it('treats a plural message reached without a count as zero', () => {
		expect(renderMessage(CY_CATS, 'cy', {})).toBe('0 cathod');
	});
});

describe('a bound translator', () => {
	it('carries its locale into every formatter', () => {
		const en = createTranslator('en');
		const noon = new Date('2026-03-05T14:30:00Z');

		expect(en.locale).toBe('en');
		expect(en.t('settings.title')).toBe('Settings');
		expect(en.formatNumber(1234.5)).toBe('1,234.5');
		expect(en.formatPercent(0.4)).toBe('40%');
		expect(en.formatDate(noon, { timeZone: 'UTC' })).toContain('2026');
		expect(en.formatDateTime(noon, { timeZone: 'UTC' })).toContain('2026');
		expect(en.formatRelativeTime('2026-03-04T14:30:00Z', noon)).toBe('yesterday');
	});
});
