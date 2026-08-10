import { describe, it, expect } from 'vitest';

import { toArrowCatalogRecords, toInitialRuntimeUpdates } from './arrow';

describe('toArrowCatalogRecords', () => {
	it('reads icon and banner from the nested media object', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'github.com/x/y',
					name: 'y',
					description: 'd',
					tags: ['t'],
					media: { icon: 'i.png', banner: 'b.png' },
					versions: [
						{ ref: '1.0.0', version: '1.0.0', state: 'ready', installed_at: '2026-05-09T21:26:59Z' },
					],
				},
			],
			'local'
		);
		expect(records[0].icon).toBe('i.png');
		expect(records[0].banner).toBe('b.png');
	});

	it('tolerates an absent media object', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'a',
					name: 'a',
					description: '',
					tags: [],
					versions: [{ ref: '1', version: '1', state: 'ready' }],
				},
			],
			'local'
		);
		expect(records[0].icon).toBeNull();
	});

	it('stamps every record with its connection', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'a',
					name: 'a',
					description: '',
					tags: [],
					versions: [{ ref: '1', version: '1', state: 'ready' }],
				},
			],
			'remote-7'
		);
		expect(records[0].connectionId).toBe('remote-7');
	});

	it('produces one record per installed version', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'a',
					name: 'a',
					description: '',
					tags: [],
					versions: [
						{ ref: '1', version: '1', state: 'ready' },
						{ ref: '2', version: '2', state: 'absent' },
					],
				},
			],
			'local'
		);
		expect(records.map((r) => r.namespace)).toEqual(['a@1', 'a@2']);
	});
});

describe('toInitialRuntimeUpdates', () => {
	it('carries versions[].state through as the initial state', () => {
		const updates = toInitialRuntimeUpdates([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'running' }],
			},
		]);
		expect(updates).toEqual([{ namespace: 'a@1', state: 'running', active_run: null, last_return: null }]);
	});

	it('produces one update per installed version, matching the catalog namespace scheme', () => {
		const updates = toInitialRuntimeUpdates([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [
					{ ref: '1', version: '1', state: 'ready' },
					{ ref: '2', version: '2', state: 'absent' },
				],
			},
		]);
		expect(updates.map((u) => u.namespace)).toEqual(['a@1', 'a@2']);
	});

	it('always nulls active_run and last_return, since the list endpoint never carries them', () => {
		const updates = toInitialRuntimeUpdates([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'running' }],
			},
		]);
		expect(updates[0].active_run).toBeNull();
		expect(updates[0].last_return).toBeNull();
	});
});
