import { describe, it, expect } from 'vitest';

import type { CollectionArrow, CollectionDetail } from './collection';

describe('CollectionArrow', () => {
	it('allows a resolved member with a name and description', () => {
		const arrow: CollectionArrow = {
			namespace: 'github.com/rabbyte/minecraft',
			version: 'v1.21.4',
			resolved: true,
			name: 'Minecraft Server',
			description: 'Vanilla dedicated server.',
		};
		expect(arrow.resolved).toBe(true);
	});

	it('allows an unresolved member with no name or description', () => {
		const arrow: CollectionArrow = {
			namespace: 'github.com/rabbyte/ark-survival',
			version: 'v3.1.0',
			resolved: false,
		};
		expect(arrow.name).toBeUndefined();
	});
});

describe('CollectionDetail', () => {
	it('carries media as a plain object, never undefined', () => {
		const detail: CollectionDetail = {
			namespace: 'ns',
			name: 'Col',
			description: '',
			tags: [],
			followed: false,
			arrowCount: 0,
			maintainers: [],
			media: {},
			arrows: [],
		};
		expect(detail.media).toEqual({});
	});
});
