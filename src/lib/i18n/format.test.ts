import { beforeEach, describe, expect, it } from 'vitest';

import {
	clearFormatterCache,
	dateFormatter,
	formatDate,
	formatDateTime,
	formatNumber,
	formatPercent,
	formatRelativeTime,
	numberFormatter,
	pluralRules,
	relativeTimeFormatter,
} from './format';

beforeEach(() => {
	clearFormatterCache();
});

// Asserted against the SHAPE the locale imposes rather than against a literal
// rendering wherever a literal would be an ICU-version hostage. `1,234.5` and
// `1.234,5` have been those two things since CLDR existed; the exact spelling
// of a medium date has not, and pinning it is how a suite starts failing on a
// Node upgrade that changed nothing anyone cares about.

describe('numbers', () => {
	it('uses the separators the reader expects', () => {
		expect(formatNumber(1234.5, 'en')).toBe('1,234.5');
		expect(formatNumber(1234.5, 'de-DE')).toBe('1.234,5');
	});

	it('takes the same options Intl does', () => {
		expect(formatNumber(0.5, 'en', { minimumFractionDigits: 2 })).toBe('0.50');
	});
});

describe('percentages', () => {
	// The trap the signature is named for. A caller holding the chaos slider's
	// 0–100 has to divide, and this is what happens when they forget.
	it('takes a fraction, and multiplies a percentage into nonsense', () => {
		expect(formatPercent(0.4, 'en')).toBe('40%');
		expect(formatPercent(40, 'en')).toBe('4,000%');
	});

	// Worth going through Intl at all for what looks like `${n}%`: not every
	// locale writes the symbol tight against the digits.
	it('places the symbol the way the locale does', () => {
		expect(formatPercent(0.4, 'fr-FR')).not.toBe(formatPercent(0.4, 'en'));
	});
});

describe('dates', () => {
	const noon = new Date('2026-03-05T14:30:00Z');

	it('orders the parts the way the locale does', () => {
		const us = formatDate(noon, 'en-US', { timeZone: 'UTC' });
		const gb = formatDate(noon, 'en-GB', { timeZone: 'UTC' });

		// Same language, same words, opposite order — which is the whole reason
		// a date cannot be assembled by hand.
		expect(us.indexOf('Mar')).toBeLessThan(us.indexOf('5'));
		expect(gb.indexOf('5')).toBeLessThan(gb.indexOf('Mar'));
	});

	// The trap named in `formatDate`'s comment, demonstrated rather than
	// described: the same instant is two different days depending on where you
	// are standing, so a test that omits `timeZone` passes in CI and fails in
	// Denver.
	it('renders one instant as two different dates in two zones', () => {
		const nearMidnight = new Date('2026-03-05T02:00:00Z');
		expect(formatDate(nearMidnight, 'en-US', { timeZone: 'UTC' })).toContain('5');
		expect(formatDate(nearMidnight, 'en-US', { timeZone: 'America/Denver' })).toContain('4');
	});

	it('accepts an ISO string or an epoch as readily as a Date', () => {
		const fromDate = formatDate(noon, 'en-US', { timeZone: 'UTC' });
		expect(formatDate('2026-03-05T14:30:00Z', 'en-US', { timeZone: 'UTC' })).toBe(fromDate);
		expect(formatDate(noon.getTime(), 'en-US', { timeZone: 'UTC' })).toBe(fromDate);
	});

	it('adds a time when asked for one', () => {
		const stamped = formatDateTime(noon, 'en-US', { timeZone: 'UTC' });
		expect(stamped).toContain('2026');
		expect(stamped).toMatch(/\d:\d\d/);
	});
});

describe('relative time', () => {
	const now = new Date('2026-03-05T14:30:00Z');
	const ago = (ms: number) => formatRelativeTime(new Date(now.getTime() - ms), 'en', now);

	it('says it in the largest unit that fits', () => {
		expect(ago(45 * 1000)).toBe('45 seconds ago');
		expect(ago(90 * 1000)).toBe('1 minute ago');
		expect(ago(3 * 60 * 60 * 1000)).toBe('3 hours ago');
		expect(ago(10 * 24 * 60 * 60 * 1000)).toBe('last week');
		expect(ago(60 * 24 * 60 * 60 * 1000)).toBe('2 months ago');
		expect(ago(400 * 24 * 60 * 60 * 1000)).toBe('last year');
	});

	it('keeps the direction, so the future does not read as the past', () => {
		expect(ago(-3 * 60 * 60 * 1000)).toBe('in 3 hours');
	});

	// `numeric: 'auto'` is the default here, so the language gets to use its own
	// word where it has one. "1 day ago" is understood; "yesterday" is written.
	it('prefers the word over the number where the language has one', () => {
		expect(ago(24 * 60 * 60 * 1000)).toBe('yesterday');
		expect(ago(-24 * 60 * 60 * 1000)).toBe('tomorrow');
	});

	it('collapses anything under a second to "now"', () => {
		expect(ago(0)).toBe('now');
		expect(ago(500)).toBe('now');
	});

	it('defaults its reference to the present', () => {
		expect(formatRelativeTime(Date.now(), 'en')).toBe('now');
	});
});

describe('the formatter cache', () => {
	// Construction is the expensive half of Intl and formatting is the cheap
	// one, so a panel that formats one value per row must not build one
	// formatter per row per render.
	it('hands back the same instance for the same locale and options', () => {
		expect(numberFormatter('en')).toBe(numberFormatter('en'));
		expect(dateFormatter('en', { dateStyle: 'medium' })).toBe(dateFormatter('en', { dateStyle: 'medium' }));
		expect(relativeTimeFormatter('en')).toBe(relativeTimeFormatter('en'));
		expect(pluralRules('ru')).toBe(pluralRules('ru'));
	});

	it('does not let one locale or one set of options answer for another', () => {
		expect(numberFormatter('en')).not.toBe(numberFormatter('de-DE'));
		expect(numberFormatter('en')).not.toBe(numberFormatter('en', { style: 'percent' }));
	});

	it('is emptied by the test seam, so state cannot leak between cases', () => {
		const before = numberFormatter('en');
		clearFormatterCache();
		expect(numberFormatter('en')).not.toBe(before);
	});
});
