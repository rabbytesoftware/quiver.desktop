import { describe, expect, it } from 'vitest';

import type { ArrowEntry } from '@/domain/arrow';

import { buildDependencyRows } from './dependency-rows';

function entry(overrides: Partial<ArrowEntry> = {}): ArrowEntry {
	return {
		namespace: 'github.com/rabbyte/nats@v2.10.0',
		name: 'NATS',
		description: '',
		tags: [],
		icon: 'nats-icon.png',
		banner: null,
		version: '2.10.0',
		state: 'ready',
		active_run: null,
		last_return: null,
		...overrides,
	};
}

describe('buildDependencyRows', () => {
	it('enriches a namespace with the exact-ref catalog entry, when one exists', () => {
		const catalog = new Map([[entry().namespace, entry()]]);
		const [row] = buildDependencyRows(['github.com/rabbyte/nats@v2.10.0'], catalog);

		expect(row).toEqual({
			namespace: 'github.com/rabbyte/nats@v2.10.0',
			name: 'NATS',
			icon: 'nats-icon.png',
			ref: 'v2.10.0',
			state: 'ready',
			userInstalled: true,
		});
	});

	it('falls back to any installed ref sharing the bare namespace when the exact ref is not in the catalog', () => {
		const catalog = new Map([[entry().namespace, entry()]]);
		const [row] = buildDependencyRows(['github.com/rabbyte/nats@v2.11.0'], catalog);

		expect(row.name).toBe('NATS');
		expect(row.icon).toBe('nats-icon.png');
		expect(row.ref).toBe('v2.11.0');
		expect(row.userInstalled).toBe(true);
	});

	it('falls back to the bare namespace as the name, with no icon and userInstalled false, when nothing matches', () => {
		const [row] = buildDependencyRows(['github.com/rabbyte/redis@v7.4.1'], new Map());

		expect(row).toEqual({
			namespace: 'github.com/rabbyte/redis@v7.4.1',
			name: 'github.com/rabbyte/redis',
			icon: null,
			ref: 'v7.4.1',
			state: 'absent',
			userInstalled: false,
		});
	});

	it('preserves order and maps every namespace given', () => {
		const rows = buildDependencyRows(['a/one@v1', 'a/two@v1', 'a/three@v1'], new Map());
		expect(rows.map((row) => row.namespace)).toEqual(['a/one@v1', 'a/two@v1', 'a/three@v1']);
	});
});
