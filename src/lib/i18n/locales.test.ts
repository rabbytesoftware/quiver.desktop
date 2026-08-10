import { describe, expect, it } from 'vitest';

import { CATALOGUES, LOCALES, SOURCE_LOCALE } from './catalogue';
import { en } from './locales/en';
import type { Catalogue, Message } from './types';

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

function formsOf(message: Message): string[] {
	if (typeof message === 'string') return [message];
	return Object.values(message).filter((form): form is string => typeof form === 'string');
}

function holesIn(form: string): Set<string> {
	return new Set([...form.matchAll(/\{(\w+)\}/g)].map(([, name]) => name));
}

function holes(message: Message): Set<string> {
	const found = new Set<string>();
	for (const form of formsOf(message)) for (const name of holesIn(form)) found.add(name);
	return found;
}

function sameHoles(source: Message, target: Message): boolean {
	const required = holes(source);
	return formsOf(target).every((form) => {
		const found = holesIn(form);
		return found.size === required.size && [...required].every((name) => found.has(name));
	});
}

const NO_DIVERGENCE: Divergence = { missing: [], extra: [], shape: [], placeholders: [] };

describe('every shipped catalogue', () => {
	it.each([...LOCALES])('%s answers for every key in the source locale', (locale) => {
		expect(compare(en, CATALOGUES[locale])).toEqual(NO_DIVERGENCE);
	});

	it('registers the source locale it falls back to', () => {
		expect(LOCALES).toContain(SOURCE_LOCALE);
	});
});

describe('the guard itself', () => {
	it('reports a key the translation never got to', () => {
		const { 'settings.title': _dropped, ...incomplete } = en;
		expect(compare(en, incomplete).missing).toEqual(['settings.title']);
	});

	it('reports a key the source locale has since dropped', () => {
		expect(compare(en, { ...en, 'settings.removed': 'left behind' }).extra).toEqual(['settings.removed']);
	});

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
