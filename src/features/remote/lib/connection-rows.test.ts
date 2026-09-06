import { describe, it, expect } from 'vitest';

import type { ConnectionConfig } from '@/domain/connection';

import { connectionRows } from './connection-rows';

const local: ConnectionConfig = { id: 'local', name: 'Local', kind: 'local', api_version: 'v0' };
const homeLab: ConnectionConfig = {
	id: 'home-lab',
	name: 'Home Lab',
	kind: 'remote',
	url: 'http://192.168.1.42:7420',
	api_version: 'v0',
};

describe('connectionRows', () => {
	it('marks the row matching activeId as active and the rest as not', () => {
		const rows = connectionRows([local, homeLab], 'local', 'ready');
		expect(rows.find((r) => r.id === 'local')?.isActive).toBe(true);
		expect(rows.find((r) => r.id === 'home-lab')?.isActive).toBe(false);
	});

	it('gives the local row no subtitle and a remote row its url as subtitle', () => {
		const rows = connectionRows([local, homeLab], 'local', 'ready');
		expect(rows.find((r) => r.id === 'local')?.subtitle).toBeNull();
		expect(rows.find((r) => r.id === 'home-lab')?.subtitle).toBe('http://192.168.1.42:7420');
	});

	it('only shows a live status on the active row', () => {
		const rows = connectionRows([local, homeLab], 'home-lab', 'starting');
		expect(rows.find((r) => r.id === 'home-lab')?.statusKind).toBe('starting');
		expect(rows.find((r) => r.id === 'local')?.statusKind).toBeNull();
	});

	it('only offers "Connect" on rows that are not already active', () => {
		const rows = connectionRows([local, homeLab], 'local', 'ready');
		expect(rows.find((r) => r.id === 'local')?.showConnect).toBe(false);
		expect(rows.find((r) => r.id === 'home-lab')?.showConnect).toBe(true);
	});

	/** Local can never be renamed or removed, but it can still be switched away
	 *  from -- so its menu button shows once it is no longer the active one. */
	it('shows the row menu for a remote regardless of active state, and for local only when inactive', () => {
		const whenLocalActive = connectionRows([local, homeLab], 'local', 'ready');
		expect(whenLocalActive.find((r) => r.id === 'local')?.showMenuBtn).toBe(false);
		expect(whenLocalActive.find((r) => r.id === 'home-lab')?.showMenuBtn).toBe(true);

		const whenHomeLabActive = connectionRows([local, homeLab], 'home-lab', 'ready');
		expect(whenHomeLabActive.find((r) => r.id === 'local')?.showMenuBtn).toBe(true);
		expect(whenHomeLabActive.find((r) => r.id === 'home-lab')?.showMenuBtn).toBe(true);
	});

	/** With nothing to switch between, a "Local -- Connected" row is just
	 *  noise above the empty state's own prompt to add one. */
	it('omits the local row entirely when there are no saved remotes', () => {
		const rows = connectionRows([local], 'local', 'ready');
		expect(rows).toHaveLength(0);
	});

	it('keeps the local row once at least one remote is saved', () => {
		const rows = connectionRows([local, homeLab], 'local', 'ready');
		expect(rows.map((r) => r.id)).toEqual(['local', 'home-lab']);
	});
});
