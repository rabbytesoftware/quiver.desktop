import { describe, it, expect } from 'vitest';

import { toCollectionArrow, toCollectionListItem, toCollectionDetail } from './collection';

describe('toCollectionArrow', () => {
	it('splits the version out of a versioned namespace', () => {
		const arrow = toCollectionArrow({
			namespace: 'github.com/rabbyte/minecraft@v1.21.4',
			resolved: true,
			name: 'Minecraft Server',
		});
		expect(arrow.namespace).toBe('github.com/rabbyte/minecraft');
		expect(arrow.version).toBe('v1.21.4');
		expect(arrow.name).toBe('Minecraft Server');
	});

	it('leaves a bare namespace without a version alone', () => {
		const arrow = toCollectionArrow({ namespace: 'github.com/rabbyte/minecraft', resolved: true });
		expect(arrow.namespace).toBe('github.com/rabbyte/minecraft');
		expect(arrow.version).toBeUndefined();
	});

	it("carries the member's own icon and banner, and drops empty ones", () => {
		const withMedia = toCollectionArrow({
			namespace: 'github.com/rabbyte/minecraft',
			resolved: true,
			media: { icon: 'https://example.com/icon.svg', banner: 'https://example.com/banner.png' },
		});
		expect(withMedia.icon).toBe('https://example.com/icon.svg');
		expect(withMedia.banner).toBe('https://example.com/banner.png');

		const withoutMedia = toCollectionArrow({
			namespace: 'github.com/rabbyte/minecraft',
			resolved: true,
			media: { icon: '', banner: '' },
		});
		expect(withoutMedia.icon).toBeUndefined();
		expect(withoutMedia.banner).toBeUndefined();
	});

	it('carries an unresolved member through with no name or description', () => {
		const arrow = toCollectionArrow({ namespace: 'github.com/rabbyte/ark-survival@v3.1.0', resolved: false });
		expect(arrow.resolved).toBe(false);
		expect(arrow.name).toBeUndefined();
		expect(arrow.description).toBeUndefined();
	});
});

describe('toCollectionListItem', () => {
	it('maps required fields', () => {
		const item = toCollectionListItem({ namespace: 'ns', name: 'Col', arrow_count: 3, followed: true });
		expect(item).toEqual({
			namespace: 'ns',
			name: 'Col',
			description: '',
			tags: [],
			followed: true,
			arrowCount: 3,
		});
	});

	it('defaults description and tags when absent', () => {
		const item = toCollectionListItem({ namespace: 'ns', name: 'Col', arrow_count: 0, followed: false });
		expect(item.description).toBe('');
		expect(item.tags).toEqual([]);
	});
});

describe('toCollectionDetail', () => {
	it('derives arrowCount from the arrows array length', () => {
		const detail = toCollectionDetail({
			namespace: 'ns',
			name: 'Col',
			followed: true,
			arrows: [
				{ namespace: 'a@1', resolved: true, name: 'A' },
				{ namespace: 'b@1', resolved: false },
			],
		});
		expect(detail.arrowCount).toBe(2);
	});

	it('defaults media to an empty-valued object when absent', () => {
		const detail = toCollectionDetail({ namespace: 'ns', name: 'Col', followed: false, arrows: [] });
		expect(detail.media).toEqual({ icon: undefined, banner: undefined });
	});

	it('preserves url and maintainers when present', () => {
		const detail = toCollectionDetail({
			namespace: 'ns',
			name: 'Col',
			followed: false,
			url: 'https://github.com/rabbyte/game-servers',
			maintainers: ['rabbyte'],
			arrows: [],
		});
		expect(detail.url).toBe('https://github.com/rabbyte/game-servers');
		expect(detail.maintainers).toEqual(['rabbyte']);
	});

	it('defaults maintainers to an empty array when absent', () => {
		const detail = toCollectionDetail({ namespace: 'ns', name: 'Col', followed: false, arrows: [] });
		expect(detail.maintainers).toEqual([]);
	});
});
