import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useRemoteStore } from './remote-store';

const initial = useRemoteStore.getState();

beforeEach(() => {
	useRemoteStore.setState(initial, true);
});

describe('add dialog', () => {
	it('opens and closes', () => {
		useRemoteStore.getState().openAdd();
		expect(useRemoteStore.getState().addOpen).toBe(true);
		useRemoteStore.getState().closeAdd();
		expect(useRemoteStore.getState().addOpen).toBe(false);
	});
});

describe('rename dialog', () => {
	it('opens for a given connection id and closes back to null', () => {
		useRemoteStore.getState().openRename('home-lab');
		expect(useRemoteStore.getState().renameId).toBe('home-lab');
		useRemoteStore.getState().closeRename();
		expect(useRemoteStore.getState().renameId).toBeNull();
	});
});

describe('remove dialog', () => {
	it('opens for a given connection id and closes back to null', () => {
		useRemoteStore.getState().openRemove('home-lab');
		expect(useRemoteStore.getState().removeId).toBe('home-lab');
		useRemoteStore.getState().closeRemove();
		expect(useRemoteStore.getState().removeId).toBeNull();
	});
});

describe('row menu', () => {
	it('toggles a row menu open and closed', () => {
		useRemoteStore.getState().toggleMenu('home-lab');
		expect(useRemoteStore.getState().openMenuId).toBe('home-lab');
		useRemoteStore.getState().toggleMenu('home-lab');
		expect(useRemoteStore.getState().openMenuId).toBeNull();
	});

	it('opening one row menu closes whichever other one was open', () => {
		useRemoteStore.getState().toggleMenu('home-lab');
		useRemoteStore.getState().toggleMenu('garage');
		expect(useRemoteStore.getState().openMenuId).toBe('garage');
	});

	it('closeMenu clears whatever is open', () => {
		useRemoteStore.getState().toggleMenu('home-lab');
		useRemoteStore.getState().closeMenu();
		expect(useRemoteStore.getState().openMenuId).toBeNull();
	});
});

describe('command palette', () => {
	it('opens with a cleared query and index', () => {
		useRemoteStore.setState({ cmdQuery: 'stale', cmdIndex: 3 });
		useRemoteStore.getState().openCmd();
		expect(useRemoteStore.getState().cmdOpen).toBe(true);
		expect(useRemoteStore.getState().cmdQuery).toBe('');
		expect(useRemoteStore.getState().cmdIndex).toBe(0);
	});

	it('does not reset an in-progress query when opened again while already open', () => {
		useRemoteStore.getState().openCmd();
		useRemoteStore.getState().setCmdQuery('hom');
		useRemoteStore.getState().openCmd();
		expect(useRemoteStore.getState().cmdQuery).toBe('hom');
	});

	/** The palette is a global switcher: opening it must not leave some other
	 *  overlay (a dialog mid-submit, a row menu) stacked underneath it. */
	it('opening the palette closes every other overlay', () => {
		useRemoteStore.getState().openAdd();
		useRemoteStore.getState().openRename('home-lab');
		useRemoteStore.getState().openRemove('home-lab');
		useRemoteStore.getState().toggleMenu('home-lab');

		useRemoteStore.getState().openCmd();

		const state = useRemoteStore.getState();
		expect(state.addOpen).toBe(false);
		expect(state.renameId).toBeNull();
		expect(state.removeId).toBeNull();
		expect(state.openMenuId).toBeNull();
	});

	it('setCmdQuery resets the highlighted index back to the top', () => {
		useRemoteStore.getState().setCmdIndex(2);
		useRemoteStore.getState().setCmdQuery('ho');
		expect(useRemoteStore.getState().cmdIndex).toBe(0);
	});
});

describe('toasts', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});
	afterEach(() => {
		vi.useRealTimers();
	});

	it('adds a toast with the given message', () => {
		useRemoteStore.getState().pushToast('Connected to Home Lab');
		expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(['Connected to Home Lab']);
	});

	it('assigns each toast its own id, even for identical messages back to back', () => {
		useRemoteStore.getState().pushToast('Added Home Lab');
		useRemoteStore.getState().pushToast('Added Home Lab');
		const ids = useRemoteStore.getState().toasts.map((t) => t.id);
		expect(new Set(ids).size).toBe(2);
	});

	it('dismisses a toast by id', () => {
		useRemoteStore.getState().pushToast('Removed Garage Server');
		const id = useRemoteStore.getState().toasts[0].id;
		useRemoteStore.getState().dismissToast(id);
		expect(useRemoteStore.getState().toasts).toEqual([]);
	});

	it('auto-dismisses a toast after its timeout', () => {
		useRemoteStore.getState().pushToast('Connected to Home Lab');
		expect(useRemoteStore.getState().toasts).toHaveLength(1);
		vi.runAllTimers();
		expect(useRemoteStore.getState().toasts).toHaveLength(0);
	});
});
