import { describe, it, expect } from 'vitest';

import { toArrowListItems } from './arrow';

describe('toArrowListItems', () => {
	it('flattens versions into individual items', () => {
		const items = toArrowListItems([
			{
				namespace: 'github.com/user/repo',
				name: 'My Arrow',
				versions: [
					{ ref: 'v1.0.0', version: '1.0.0', state: 'ready' },
					{ ref: 'v2.0.0', version: '2.0.0', state: 'ready' },
				],
			},
		]);
		expect(items).toHaveLength(2);
		expect(items[0].namespace).toBe('github.com/user/repo@v1.0.0');
		expect(items[1].namespace).toBe('github.com/user/repo@v2.0.0');
	});

	it('sets active_run and last_outcome to null', () => {
		const items = toArrowListItems([
			{
				namespace: 'ns',
				name: 'X',
				versions: [{ ref: 'v1', version: '1.0', state: 'ready' }],
			},
		]);
		expect(items[0].active_run).toBeNull();
		expect(items[0].last_outcome).toBeNull();
	});
});
