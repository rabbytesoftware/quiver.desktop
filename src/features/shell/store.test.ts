import { beforeEach, describe, expect, it } from 'vitest';

import {
	normaliseSide,
	normaliseWidth,
	SHELL_STORAGE_KEY,
	SIDEBAR_DEFAULT,
	SIDEBAR_MAX,
	SIDEBAR_MIN,
	useShellStore,
} from './store';

function saved(state: Record<string, unknown>): void {
	localStorage.setItem(SHELL_STORAGE_KEY, JSON.stringify({ state, version: 0 }));
}

beforeEach(() => {
	useShellStore.setState({ sidebarSide: 'left', sidebarWidth: SIDEBAR_DEFAULT });
	localStorage.removeItem(SHELL_STORAGE_KEY);
});

describe('the shell store out of the box', () => {
	it('docks the rail left at the design width', () => {
		expect(useShellStore.getState().sidebarSide).toBe('left');
		expect(useShellStore.getState().sidebarWidth).toBe(SIDEBAR_DEFAULT);
		expect(SIDEBAR_DEFAULT).toBe(246);
	});

	it('bounds the rail at the nav collapse point and the design ceiling', () => {
		expect(SIDEBAR_MIN).toBe(160);
		expect(SIDEBAR_MAX).toBe(320);
	});
});

describe('setting the width', () => {
	it('keeps a width the rail can actually render', () => {
		useShellStore.getState().setSidebarWidth(200);
		expect(useShellStore.getState().sidebarWidth).toBe(200);
	});

	it('clamps a drag past the near end up to the minimum', () => {
		useShellStore.getState().setSidebarWidth(40);
		expect(useShellStore.getState().sidebarWidth).toBe(SIDEBAR_MIN);
	});

	it('clamps a drag past the far end down to the maximum', () => {
		useShellStore.getState().setSidebarWidth(999);
		expect(useShellStore.getState().sidebarWidth).toBe(SIDEBAR_MAX);
	});
});

describe('setting the side', () => {
	it('moves the rail to the other edge', () => {
		useShellStore.getState().setSidebarSide('right');
		expect(useShellStore.getState().sidebarSide).toBe('right');
	});
});

describe('what reaches the disk', () => {
	it('writes both settings under the namespaced key so a reload reads them back', () => {
		useShellStore.getState().setSidebarSide('right');
		useShellStore.getState().setSidebarWidth(300);

		const persisted = JSON.parse(localStorage.getItem(SHELL_STORAGE_KEY) ?? '{}') as {
			state?: Record<string, unknown>;
		};
		expect(persisted.state?.sidebarSide).toBe('right');
		expect(persisted.state?.sidebarWidth).toBe(300);
	});

	it('persists the two settings and nothing else', () => {
		useShellStore.getState().setSidebarSide('right');

		const persisted = JSON.parse(localStorage.getItem(SHELL_STORAGE_KEY) ?? '{}') as {
			state?: Record<string, unknown>;
		};
		expect(Object.keys(persisted.state ?? {}).sort()).toEqual(['sidebarSide', 'sidebarWidth']);
	});
});

describe('rehydrating from disk', () => {
	it('reads a stored choice back', async () => {
		saved({ sidebarSide: 'right', sidebarWidth: 300 });
		await useShellStore.persist.rehydrate();

		expect(useShellStore.getState().sidebarSide).toBe('right');
		expect(useShellStore.getState().sidebarWidth).toBe(300);
	});

	it('leaves the actions callable', async () => {
		saved({ sidebarSide: 'right', sidebarWidth: 300 });
		await useShellStore.persist.rehydrate();

		useShellStore.getState().setSidebarWidth(SIDEBAR_DEFAULT);
		expect(useShellStore.getState().sidebarWidth).toBe(SIDEBAR_DEFAULT);
	});

	it('clamps a stored width from below the floor', async () => {
		saved({ sidebarSide: 'left', sidebarWidth: 60 });
		await useShellStore.persist.rehydrate();

		expect(useShellStore.getState().sidebarWidth).toBe(SIDEBAR_MIN);
	});

	it('clamps a stored width from above the ceiling', async () => {
		saved({ sidebarSide: 'left', sidebarWidth: 99999 });
		await useShellStore.persist.rehydrate();

		expect(useShellStore.getState().sidebarWidth).toBe(SIDEBAR_MAX);
	});

	it.each([
		['a string', 'wide'],
		['null', null],
		['nothing at all', undefined],
	])('falls back to the default width when the stored one is %s', async (_label, width) => {
		saved({ sidebarSide: 'left', sidebarWidth: width });
		await useShellStore.persist.rehydrate();

		expect(useShellStore.getState().sidebarWidth).toBe(SIDEBAR_DEFAULT);
	});

	it.each([['top'], [''], [null], [undefined], [0], [{ side: 'right' }]])(
		'falls back to the left edge when the stored side is %o',
		async (side) => {
			saved({ sidebarSide: side, sidebarWidth: SIDEBAR_DEFAULT });
			await useShellStore.persist.rehydrate();

			expect(useShellStore.getState().sidebarSide).toBe('left');
		}
	);
});

describe('normaliseWidth', () => {
	it('keeps a width already inside the bounds', () => {
		expect(normaliseWidth(200)).toBe(200);
	});

	it.each([
		[SIDEBAR_MIN - 1, SIDEBAR_MIN],
		[SIDEBAR_MAX + 1, SIDEBAR_MAX],
	])('clamps %i to %i', (input, expected) => {
		expect(normaliseWidth(input)).toBe(expected);
	});

	it.each([['wide'], [null], [undefined], [Number.NaN], [Number.POSITIVE_INFINITY], [{ width: 200 }]])(
		'rejects %o',
		(value) => {
			expect(normaliseWidth(value)).toBe(SIDEBAR_DEFAULT);
		}
	);
});

describe('normaliseSide', () => {
	it('keeps the two edges that are real', () => {
		expect(normaliseSide('left')).toBe('left');
		expect(normaliseSide('right')).toBe('right');
	});

	it.each([['top'], [''], [null], [undefined], [0], [{ side: 'right' }]])('rejects %o', (value) => {
		expect(normaliseSide(value)).toBe('left');
	});
});
