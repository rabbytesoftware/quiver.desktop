import { describe, expect, it } from 'vitest';

import { normaliseLevel } from './log-level';

describe('normaliseLevel', () => {
	it('leaves the four canonical levels untouched', () => {
		expect(normaliseLevel('debug')).toBe('debug');
		expect(normaliseLevel('info')).toBe('info');
		expect(normaliseLevel('warn')).toBe('warn');
		expect(normaliseLevel('error')).toBe('error');
	});

	it('maps trace onto debug', () => {
		expect(normaliseLevel('trace')).toBe('debug');
	});

	it('maps warning onto warn', () => {
		expect(normaliseLevel('warning')).toBe('warn');
	});

	it('maps fatal and panic onto error', () => {
		expect(normaliseLevel('fatal')).toBe('error');
		expect(normaliseLevel('panic')).toBe('error');
	});

	it('falls back to info for anything else, rather than throwing', () => {
		expect(normaliseLevel('silent')).toBe('info');
	});
});
