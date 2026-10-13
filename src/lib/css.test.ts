import { describe, expect, it } from 'vitest';

import { cssUrl } from './css';

describe('cssUrl', () => {
	it('wraps a plain URL in a quoted url()', () => {
		expect(cssUrl('https://example.com/banner.png')).toBe('url("https://example.com/banner.png")');
	});

	it('neutralizes a crafted URL that tries to close url( and append a declaration', () => {
		expect(cssUrl('x"); background: red; --x: "')).toBe('url("x\\"); background: red; --x: \\"")');
	});
});
