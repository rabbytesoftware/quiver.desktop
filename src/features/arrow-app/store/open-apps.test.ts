import { beforeEach, describe, expect, it } from 'vitest';

import { MAX_OPEN_APPS, admit, useOpenApps } from './open-apps';

describe('admit', () => {
	it('moves an opened app to the most recent end without duplicating', () => {
		expect(admit(['a', 'b', 'c'], 'a', null)).toEqual(['b', 'c', 'a']);
		expect(admit(['a'], 'b', null)).toEqual(['a', 'b']);
	});

	it('evicts the least recently used app, never the visible one', () => {
		const full = ['a', 'b', 'c', 'd'];
		expect(MAX_OPEN_APPS).toBe(4);
		expect(admit(full, 'e', null)).toEqual(['b', 'c', 'd', 'e']);
		expect(admit(full, 'e', 'a')).toEqual(['a', 'c', 'd', 'e']);
	});

	it('honours a custom cap', () => {
		expect(admit(['a', 'b'], 'c', null, 2)).toEqual(['b', 'c']);
	});

	it('stops evicting when nothing is evictable', () => {
		expect(admit(['a'], 'b', 'a', 0)).toEqual(['a', 'b']);
	});
});

describe('useOpenApps', () => {
	beforeEach(() => useOpenApps.setState({ order: [], frames: [], visible: null, reloads: {} }));

	it('show makes an app visible and keeps it open; hide keeps it alive', () => {
		useOpenApps.getState().show('a/b');
		expect(useOpenApps.getState()).toMatchObject({ order: ['a/b'], visible: 'a/b' });

		useOpenApps.getState().hide();
		expect(useOpenApps.getState()).toMatchObject({ order: ['a/b'], visible: null });
	});

	it('switching keeps every app in the stack', () => {
		const s = useOpenApps.getState();
		s.show('a/b');
		s.show('c/d');
		expect(useOpenApps.getState().order).toEqual(['a/b', 'c/d']);
		expect(useOpenApps.getState().visible).toBe('c/d');
	});

	it('prune removes apps whose surface is gone', () => {
		const s = useOpenApps.getState();
		s.show('a/b');
		s.show('c/d');
		s.prune(new Set(['c/d']));
		expect(useOpenApps.getState().order).toEqual(['c/d']);
		expect(useOpenApps.getState().frames).toEqual(['c/d']);
	});

	it('showing an app again reorders the LRU but never the frames', () => {
		const s = useOpenApps.getState();
		s.show('a');
		s.show('b');
		s.show('c');
		s.show('a');
		s.show('b');
		expect(useOpenApps.getState().order).toEqual(['c', 'a', 'b']);
		expect(useOpenApps.getState().frames).toEqual(['a', 'b', 'c']);
	});

	it('eviction drops the least recently used frame and keeps the others in place', () => {
		const s = useOpenApps.getState();
		for (const ns of ['a', 'b', 'c', 'd']) s.show(ns);
		s.show('a');
		s.show('e');
		expect(useOpenApps.getState().order).toEqual(['c', 'd', 'a', 'e']);
		expect(useOpenApps.getState().frames).toEqual(['a', 'c', 'd', 'e']);
	});

	it('an evicted app reopens as a fresh frame at the end', () => {
		const s = useOpenApps.getState();
		for (const ns of ['a', 'b', 'c', 'd', 'e']) s.show(ns);
		expect(useOpenApps.getState().frames).toEqual(['b', 'c', 'd', 'e']);
		s.show('a');
		expect(useOpenApps.getState().frames).toEqual(['c', 'd', 'e', 'a']);
	});

	it('prune leaves the store untouched when nothing changed', () => {
		useOpenApps.getState().show('a/b');
		const before = useOpenApps.getState().order;
		useOpenApps.getState().prune(new Set(['a/b']));
		expect(useOpenApps.getState().order).toBe(before);
	});

	it('reload bumps only that app\'s key', () => {
		const s = useOpenApps.getState();
		s.reload('a/b');
		s.reload('a/b');
		s.reload('c/d');
		expect(useOpenApps.getState().reloads).toEqual({ 'a/b': 2, 'c/d': 1 });
	});

	it('prune drops the reload keys of pruned apps', () => {
		const s = useOpenApps.getState();
		s.show('a/b');
		s.show('c/d');
		s.reload('a/b');
		s.reload('c/d');
		s.prune(new Set(['c/d']));
		expect(useOpenApps.getState().reloads).toEqual({ 'c/d': 1 });
	});
});
