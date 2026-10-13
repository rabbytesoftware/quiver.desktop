import { describe, it, expect } from 'vitest';

import { toHome } from './home';

const arrowDTO = {
	namespace: 'github.com/a/b',
	name: 'B',
	description: 'd',
	tags: [],
	versions: ['v1'],
	compatible_os: [],
	installed: false,
	known: true,
	stars: 3,
};

describe('toHome', () => {
	it('maps shelves and their arrows through toSearchEntry', () => {
		const home = toHome({
			shelves: [{ id: 'popular', title: 'Popular', refreshed_at: '2026-09-30T12:00:00Z', arrows: [arrowDTO] }],
			refreshing: true,
		});
		expect(home.refreshing).toBe(true);
		expect(home.shelves).toHaveLength(1);
		expect(home.shelves[0]).toMatchObject({ id: 'popular', title: 'Popular', refreshedAt: '2026-09-30T12:00:00Z' });
		expect(home.shelves[0].arrows[0]).toMatchObject({ namespace: 'github.com/a/b', stars: 3, installed: false });
	});

	it('keeps a never-filled shelf as a null timestamp with no arrows', () => {
		const home = toHome({
			shelves: [{ id: 'x', title: 'X', refreshed_at: null, arrows: null }],
			refreshing: false,
		});
		expect(home.shelves[0].refreshedAt).toBeNull();
		expect(home.shelves[0].arrows).toEqual([]);
	});

	it('treats null shelves as none', () => {
		expect(toHome({ shelves: null, refreshing: false })).toEqual({ shelves: [], refreshing: false });
	});

	it('reads a missing refreshing flag as false', () => {
		expect(toHome({ shelves: [] } as never).refreshing).toBe(false);
	});
});
