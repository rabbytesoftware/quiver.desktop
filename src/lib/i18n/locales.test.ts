import { describe, expect, it } from 'vitest';

import { CATALOGUES, LOCALES, SOURCE_LOCALE } from './catalogue';
import { en } from './locales/en';
import type { Catalogue, Message } from './types';

/**
 * The guard: every registered catalogue answers for every key the source
 * locale defines, with the same shape and the same holes in it.
 *
 * `LocaleCatalogue` already makes a missing key a `tsc` error, so why check it
 * again at runtime? Because the type only binds a file that declares
 * `satisfies LocaleCatalogue`, and the three ways a catalogue arrives without
 * that are all plausible: a contributor copies `en.ts` and copies the
 * `satisfies Catalogue` on the last line with it — which type-checks and
 * enforces nothing — or writes `as LocaleCatalogue`, which silences the error
 * rather than answering it, or imports the JSON a translation service exported.
 * This test does not care how the object got there.
 *
 * It also catches three things no type can:
 *
 *  - a translated key that no longer exists in `en`, which is dead weight a
 *    translator will keep maintaining;
 *  - a plural message flattened to a string (or the reverse). That compiles —
 *    `Message` is the union — and the effect is that a language with four
 *    plural categories renders one of them for every count;
 *  - a `{placeholder}` dropped or misspelled in translation. That compiles too,
 *    and the value simply vanishes from the sentence: "Quiver" with no version
 *    after it, and nothing anywhere says why.
 */

interface Divergence {
	missing: string[];
	extra: string[];
	shape: string[];
	placeholders: string[];
}

function compare(source: Catalogue, target: Catalogue): Divergence {
	const sourceKeys = Object.keys(source);
	const targetKeys = new Set(Object.keys(target));

	const divergence: Divergence = { missing: [], extra: [], shape: [], placeholders: [] };

	for (const key of sourceKeys) {
		if (!targetKeys.has(key)) {
			divergence.missing.push(key);
			continue;
		}
		if (isPlural(source[key]) !== isPlural(target[key])) divergence.shape.push(key);
		else if (!sameHoles(source[key], target[key])) divergence.placeholders.push(key);
	}

	divergence.extra = [...targetKeys].filter((key) => !(key in source));
	return divergence;
}

function isPlural(message: Message): boolean {
	return typeof message !== 'string';
}

/** A plain message is one form; a plural message is as many as it declares. */
function formsOf(message: Message): string[] {
	if (typeof message === 'string') return [message];
	return Object.values(message).filter((form): form is string => typeof form === 'string');
}

function holesIn(form: string): Set<string> {
	return new Set([...form.matchAll(/\{(\w+)\}/g)].map(([, name]) => name));
}

/** Every `{name}` a message can interpolate, across all of its forms. */
function holes(message: Message): Set<string> {
	const found = new Set<string>();
	for (const form of formsOf(message)) for (const name of holesIn(form)) found.add(name);
	return found;
}

/**
 * EVERY FORM of the translation must carry EVERY placeholder the source has —
 * not the union across forms, which is the weaker check and the one that misses
 * the interesting bug.
 *
 * The invariant is that a message renders every value it was handed whichever
 * form gets picked. A `one` form that lost its `{count}` renders fine for
 * count 1 and drops the number for nobody, because in English count 1 is the
 * only value that reaches it — and then Russian arrives, where `one` is also
 * 21, 31 and 101, and the number vanishes for all of them.
 *
 * It does reject "one more tap…" with the digit written out, which is a
 * defensible translation. That is the intended trade: it is indistinguishable
 * from a form that lost its placeholder in a copy-paste, and the second is by
 * far the more common way to arrive at it.
 */
function sameHoles(source: Message, target: Message): boolean {
	const required = holes(source);
	return formsOf(target).every((form) => {
		const found = holesIn(form);
		return found.size === required.size && [...required].every((name) => found.has(name));
	});
}

const NO_DIVERGENCE: Divergence = { missing: [], extra: [], shape: [], placeholders: [] };

describe('every shipped catalogue', () => {
	// `it.each` over the registry rather than a loop with one assertion: a
	// failure then names the locale in the test title, which is the first thing
	// anyone reads off CI.
	it.each([...LOCALES])('%s answers for every key in the source locale', (locale) => {
		expect(compare(en, CATALOGUES[locale])).toEqual(NO_DIVERGENCE);
	});

	it('registers the source locale it falls back to', () => {
		expect(LOCALES).toContain(SOURCE_LOCALE);
	});
});

// The guard above passes trivially while `en` is the only catalogue. These
// cases are what make it a guard rather than a tautology: they show it FAILING
// on each kind of gap, so the day a second locale lands the assertion above is
// known to mean something.
describe('the guard itself', () => {
	it('reports a key the translation never got to', () => {
		const { 'settings.title': _dropped, ...incomplete } = en;
		expect(compare(en, incomplete).missing).toEqual(['settings.title']);
	});

	it('reports a key the source locale has since dropped', () => {
		expect(compare(en, { ...en, 'settings.removed': 'left behind' }).extra).toEqual(['settings.removed']);
	});

	// The one that types allow and users notice: every count rendered with the
	// same noun.
	it('reports a plural message flattened to a single string', () => {
		const flattened = { ...en, 'settings.version.remaining': '{count} more taps…' };
		expect(compare(en, flattened).shape).toEqual(['settings.version.remaining']);
	});

	it('reports a placeholder dropped in translation', () => {
		const dropped = { ...en, 'settings.version.text': 'Quiver' };
		expect(compare(en, dropped).placeholders).toEqual(['settings.version.text']);
	});

	it('reports a placeholder misspelled in translation', () => {
		const typo = { ...en, 'settings.version.text': 'Quiver {versoin}' };
		expect(compare(en, typo).placeholders).toEqual(['settings.version.text']);
	});

	// A plural form that quietly loses `{count}` is the same defect one level
	// down, and a comparison that only looked at `other` would miss it.
	it('looks inside every plural form, not only `other`', () => {
		const lopsided = {
			...en,
			'settings.version.remaining': { one: 'one more tap…', other: '{count} more taps…' },
		};
		expect(compare(en, lopsided).placeholders).toEqual(['settings.version.remaining']);
	});
});

describe('the source catalogue', () => {
	it('names itself in its own language, for the picker', () => {
		expect(en['locale.name']).toBe('English');
	});

	it('has no empty messages', () => {
		const empty = Object.entries(en).filter(([, message]) => formsOf(message).some((form) => form.length === 0));
		expect(empty).toEqual([]);
	});
});
