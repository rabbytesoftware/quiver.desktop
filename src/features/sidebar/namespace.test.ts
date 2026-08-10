import { describe, expect, it } from 'vitest';

import { splitNamespace } from './namespace';

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

	it('splits at the last @, not the first', () => {
		expect(splitNamespace('git@host.com/x')).toEqual({ head: 'git', tail: '@host.com/x' });
		expect(splitNamespace('git@host.com/x@v2')).toEqual({ head: 'git@host.com/x', tail: '@v2' });
	});

	it('splits an empty namespace into two empty halves', () => {
		expect(splitNamespace('')).toEqual({ head: '', tail: '' });
	});

	it('handles a namespace that is nothing but a version', () => {
		expect(splitNamespace('@v1.21.4')).toEqual({ head: '', tail: '@v1.21.4' });
	});
});
