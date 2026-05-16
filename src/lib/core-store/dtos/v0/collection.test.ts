import { describe, it, expect } from 'vitest';

import { toCollectionListItem, toCollectionDetail } from './collection';

describe('toCollectionListItem', () => {
	it('maps required fields', () => {
		const item = toCollectionListItem({ namespace: 'ns', name: 'Col', arrows: [] });
		expect(item.namespace).toBe('ns');
		expect(item.name).toBe('Col');
		expect(item.arrows).toEqual([]);
	});

	it('defaults description to empty string when absent', () => {
		const item = toCollectionListItem({ namespace: 'ns', name: 'Col', arrows: [] });
		expect(item.description).toBe('');
	});

	it('preserves provided description', () => {
		const item = toCollectionListItem({
			namespace: 'ns',
			name: 'Col',
			description: 'About',
			arrows: [],
		});
		expect(item.description).toBe('About');
	});
});

describe('toCollectionDetail', () => {
	it('defaults readme to empty string when absent', () => {
		const detail = toCollectionDetail({ namespace: 'ns', name: 'Col', arrows: [] });
		expect(detail.readme).toBe('');
	});

	it('preserves provided readme', () => {
		const detail = toCollectionDetail({
			namespace: 'ns',
			name: 'Col',
			arrows: [],
			readme: '# readme',
		});
		expect(detail.readme).toBe('# readme');
	});

	it('includes all base fields', () => {
		const detail = toCollectionDetail({
			namespace: 'ns',
			name: 'Col',
			description: 'D',
			arrows: [],
		});
		expect(detail.description).toBe('D');
	});
});
