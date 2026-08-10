import { describe, expect, it } from 'vitest';

import { createTranslator, interpolate, renderMessage, selectPluralForm, translateIn } from './catalogue';
import type { PluralMessage } from './types';

const RU_ARROWS: PluralMessage = {
	one: '{count} стрела',
	few: '{count} стрелы',
	many: '{count} стрел',
	other: '{count} стрелы',
};

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

	it('renders the key itself rather than `undefined` when no message exists', () => {
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

	it('formats a numeric value for the locale, not for the developer', () => {
		expect(interpolate('{n} arrows', 'en', { n: 1234 })).toBe('1,234 arrows');
		expect(interpolate('{n} Pfeile', 'de-DE', { n: 1234 })).toBe('1.234 Pfeile');
	});

	it('does not rescan what it just substituted', () => {
		expect(interpolate('{name} is here', 'en', { name: '{name} {count}' })).toBe('{name} {count} is here');
	});

	it('leaves an unfilled hole standing, so the gap is visible', () => {
		expect(interpolate('{a} and {b}', 'en', { a: 'this' })).toBe('this and {b}');
	});

	it('ignores braces that are not a placeholder', () => {
		expect(interpolate('use { "id": 1 } here', 'en', { id: 'x' })).toBe('use { "id": 1 } here');
	});
});

describe('pluralisation', () => {
	it('picks the English form on a two-category language', () => {
		expect(translateIn('en', 'settings.version.remaining', { count: 1 })).toBe('1 more tap…');
		expect(translateIn('en', 'settings.version.remaining', { count: 3 })).toBe('3 more taps…');
		expect(translateIn('en', 'settings.version.remaining', { count: 0 })).toBe('0 more taps…');
	});

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

	it('falls back to `other` for a form the catalogue omits', () => {
		const partial: PluralMessage = { one: 'один', other: 'много' };
		expect(selectPluralForm(partial, 'ru', 1)).toBe('один');
		expect(selectPluralForm(partial, 'ru', 5)).toBe('много');
	});

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
