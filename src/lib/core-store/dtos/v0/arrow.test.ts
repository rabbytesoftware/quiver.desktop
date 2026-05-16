import { describe, it, expect } from 'vitest';
import { toArrowListItems, type ArrowListResponseItemDTO } from './arrow';

describe('toArrowListItems', () => {
	it('maps slim fields from list response', () => {
		const input: ArrowListResponseItemDTO[] = [
			{
				namespace: 'github.com/user/repo',
				name: 'My Arrow',
				description: 'A test arrow',
				tags: ['cli'],
				icon: 'https://example.com/icon.png',
				banner: null,
				versions: [{ ref: 'v1.0.0', version: '1.0.0', state: 'ready' }],
			},
		];
		const result = toArrowListItems(input);
		expect(result).toHaveLength(1);
		expect(result[0].namespace).toBe('github.com/user/repo@v1.0.0');
		expect(result[0].name).toBe('My Arrow');
		expect(result[0].description).toBe('A test arrow');
		expect(result[0].tags).toEqual(['cli']);
		expect(result[0].icon).toBe('https://example.com/icon.png');
		expect(result[0].banner).toBeNull();
		expect(result[0].state).toBe('ready');
		expect(result[0].active_run).toBeNull();
		expect(result[0].last_return).toBeNull();
	});

	it('defaults icon and banner to null when absent', () => {
		const input: ArrowListResponseItemDTO[] = [
			{
				namespace: 'github.com/user/repo',
				name: 'Arrow',
				description: '',
				tags: [],
				versions: [{ ref: 'v1', version: '1.0.0', state: 'ready' }],
			},
		];
		const result = toArrowListItems(input);
		expect(result[0].icon).toBeNull();
		expect(result[0].banner).toBeNull();
	});

	it('expands multiple versions into separate entries', () => {
		const input: ArrowListResponseItemDTO[] = [
			{
				namespace: 'github.com/user/repo',
				name: 'Arrow',
				description: '',
				tags: [],
				versions: [
					{ ref: 'v1.0.0', version: '1.0.0', state: 'ready' },
					{ ref: 'v2.0.0', version: '2.0.0', state: 'absent' },
				],
			},
		];
		const result = toArrowListItems(input);
		expect(result).toHaveLength(2);
		expect(result[0].namespace).toBe('github.com/user/repo@v1.0.0');
		expect(result[1].namespace).toBe('github.com/user/repo@v2.0.0');
	});
});
