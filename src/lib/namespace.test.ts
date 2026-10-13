import { describe, expect, it } from 'vitest';

import { bareNamespace, namespaceSegment, ownerOf, selectorOf, splitNamespace, withSelector } from './namespace';

describe('splitNamespace', () => {
	it('separates the version from the path', () => {
		expect(splitNamespace('github.com/rabbyte/minecraft@v1.21.4')).toEqual({
			head: 'github.com/rabbyte/minecraft',
			tail: '@v1.21.4',
		});
	});

	it('puts an unversioned namespace entirely in the head', () => {
		expect(splitNamespace('github.com/rabbyte/minecraft')).toEqual({
			head: 'github.com/rabbyte/minecraft',
			tail: '',
		});
	});

	it('splits at the first @, the way quiver.core does', () => {
		expect(splitNamespace('github.com/u/r@a@b')).toEqual({ head: 'github.com/u/r', tail: '@a@b' });
	});

	it('keeps a slashed selector whole in the tail', () => {
		expect(splitNamespace('github.com/u/r@feat/x')).toEqual({ head: 'github.com/u/r', tail: '@feat/x' });
	});

	it('keeps a glob selector whole in the tail', () => {
		expect(splitNamespace('github.com/char2cs/crowbar@v1.*')).toEqual({
			head: 'github.com/char2cs/crowbar',
			tail: '@v1.*',
		});
	});

	it('splits an empty namespace into two empty halves', () => {
		expect(splitNamespace('')).toEqual({ head: '', tail: '' });
	});

	it('handles a namespace that is nothing but a version', () => {
		expect(splitNamespace('@v1.21.4')).toEqual({ head: '', tail: '@v1.21.4' });
	});
});

describe('bareNamespace', () => {
	it('drops the selector', () => {
		expect(bareNamespace('github.com/char2cs/crowbar@nightly')).toBe('github.com/char2cs/crowbar');
	});

	it('returns a refless namespace unchanged', () => {
		expect(bareNamespace('github.com/char2cs/crowbar')).toBe('github.com/char2cs/crowbar');
	});
});

describe('selectorOf', () => {
	it('returns everything after the first @', () => {
		expect(selectorOf('github.com/u/r@feat/x')).toBe('feat/x');
	});

	it('is empty for a refless namespace', () => {
		expect(selectorOf('github.com/u/r')).toBe('');
	});
});

describe('withSelector', () => {
	it('replaces any selector the namespace already carries', () => {
		expect(withSelector('github.com/char2cs/crowbar@stable', 'beta')).toBe('github.com/char2cs/crowbar@beta');
	});

	it('returns the bare namespace for an empty selector', () => {
		expect(withSelector('github.com/char2cs/crowbar@stable', '')).toBe('github.com/char2cs/crowbar');
	});
});

describe('namespaceSegment', () => {
	it.each([
		['github.com/char2cs/crowbar@nightly', 'github.com%2Fchar2cs%2Fcrowbar%40nightly'],
		['github.com/u/r@feat/x', 'github.com%2Fu%2Fr%40feat%2Fx'],
		['github.com/u/r@v1.*', 'github.com%2Fu%2Fr%40v1.*'],
	])('encodes %s as one path segment', (ns, segment) => {
		expect(namespaceSegment(ns)).toBe(segment);
		expect(namespaceSegment(ns)).not.toContain('/');
	});
});

describe('ownerOf', () => {
	it('takes the segment before the repo when the host is present', () => {
		expect(ownerOf('github.com/rabbyte/minecraft')).toBe('rabbyte');
	});

	it('falls back to the first segment when there is no host', () => {
		expect(ownerOf('rabbyte/minecraft')).toBe('rabbyte');
	});

	it('falls back to the whole string when there is no slash at all', () => {
		expect(ownerOf('minecraft')).toBe('minecraft');
	});
});
