import { describe, expect, it } from 'vitest';

import { cn } from './cn';

describe('cn', () => {
	it('lets a later padding win over a custom-property padding', () => {
		expect(cn('p-(--inset)', 'p-0')).toBe('p-0');
	});

	it('dedupes custom-property sizing', () => {
		expect(cn('size-(--icon)', 'size-(--icon-nav)')).toBe('size-(--icon-nav)');
	});
});
