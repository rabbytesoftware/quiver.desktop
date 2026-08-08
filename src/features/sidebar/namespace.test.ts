import { describe, expect, it } from 'vitest';

import { splitNamespace } from './namespace';

describe('splitNamespace', () => {
	it('separates the version from the path', () => {
		expect(splitNamespace('github.com/rabbyte/minecraft@v1.21.4')).toEqual({
			head: 'github.com/rabbyte/minecraft',
			tail: '@v1.21.4',
		});
	});

	// An unversioned namespace must still render: an empty tail contributes no
	// width, so the head simply gets the whole row.
	it('puts an unversioned namespace entirely in the head', () => {
		expect(splitNamespace('github.com/rabbyte/minecraft')).toEqual({
			head: 'github.com/rabbyte/minecraft',
			tail: '',
		});
	});

	// The tail is pinned `shrink-0`. Splitting at the FIRST `@` would hand
	// `host.com/x` to it and the subtitle would refuse to truncate at all,
	// overflowing the rail instead of shedding the middle of the path.
	it('splits at the last @, not the first', () => {
		expect(splitNamespace('git@host.com/x')).toEqual({ head: 'git', tail: '@host.com/x' });
		expect(splitNamespace('git@host.com/x@v2')).toEqual({ head: 'git@host.com/x', tail: '@v2' });
	});

	it('splits an empty namespace into two empty halves', () => {
		expect(splitNamespace('')).toEqual({ head: '', tail: '' });
	});

	// `lastIndexOf` returns 0 here, which is falsy — a truthiness check in place
	// of `=== -1` would send the whole string to the head and print the version
	// where the path belongs.
	it('handles a namespace that is nothing but a version', () => {
		expect(splitNamespace('@v1.21.4')).toEqual({ head: '', tail: '@v1.21.4' });
	});
});
