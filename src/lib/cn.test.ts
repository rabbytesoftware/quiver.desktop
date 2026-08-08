import { describe, expect, it } from 'vitest';

import { cn } from './cn';

/**
 * These two cases are the reason `tailwind-merge` had to move off 1.14.0.
 *
 * That release is a Tailwind **v3** build, and this project is on Tailwind 4.
 * It cannot parse v4's custom-property value syntax — `p-(--inset)`,
 * `size-(--icon)` — so it treats them as unrecognised classes, keeps both sides
 * of a conflict, and leaves the winner to whatever order Tailwind happened to
 * emit. Callers worked around it by branching (`leading ? 'pl-0' : 'pl-[12px]'`)
 * instead of overriding, which is a workaround for a broken merge rather than a
 * design decision. Guard the behaviour so nobody reintroduces the old build.
 */
describe('cn', () => {
	it('lets a later padding win over a custom-property padding', () => {
		expect(cn('p-(--inset)', 'p-0')).toBe('p-0');
	});

	it('dedupes custom-property sizing', () => {
		expect(cn('size-(--icon)', 'size-(--icon-nav)')).toBe('size-(--icon-nav)');
	});
});
