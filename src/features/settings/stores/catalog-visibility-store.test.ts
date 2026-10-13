import { beforeEach, describe, expect, it } from 'vitest';

import {
	CATALOG_VISIBILITY_STORAGE_KEY,
	normaliseShowSelfComponents,
	useCatalogVisibilityStore,
} from './catalog-visibility-store';

function saved(state: Record<string, unknown>): void {
	localStorage.setItem(CATALOG_VISIBILITY_STORAGE_KEY, JSON.stringify({ state, version: 0 }));
}

beforeEach(() => {
	useCatalogVisibilityStore.setState({ showSelfComponents: true });
	localStorage.removeItem(CATALOG_VISIBILITY_STORAGE_KEY);
});

describe('the catalog visibility store out of the box', () => {
	it('shows Quiver’s own components by default -- opt-out, not opt-in', () => {
		expect(useCatalogVisibilityStore.getState().showSelfComponents).toBe(true);
	});
});

describe('toggling the setting', () => {
	it('hides Quiver’s own components when turned off', () => {
		useCatalogVisibilityStore.getState().setShowSelfComponents(false);
		expect(useCatalogVisibilityStore.getState().showSelfComponents).toBe(false);
	});

	it('shows them again when turned back on', () => {
		useCatalogVisibilityStore.getState().setShowSelfComponents(false);
		useCatalogVisibilityStore.getState().setShowSelfComponents(true);
		expect(useCatalogVisibilityStore.getState().showSelfComponents).toBe(true);
	});
});

describe('what reaches the disk', () => {
	it('writes the setting under the namespaced key so a reload reads it back', () => {
		useCatalogVisibilityStore.getState().setShowSelfComponents(false);

		const persisted = JSON.parse(localStorage.getItem(CATALOG_VISIBILITY_STORAGE_KEY) ?? '{}') as {
			state?: Record<string, unknown>;
		};
		expect(persisted.state?.showSelfComponents).toBe(false);
	});

	it('persists this one setting and nothing else', () => {
		useCatalogVisibilityStore.getState().setShowSelfComponents(false);

		const persisted = JSON.parse(localStorage.getItem(CATALOG_VISIBILITY_STORAGE_KEY) ?? '{}') as {
			state?: Record<string, unknown>;
		};
		expect(Object.keys(persisted.state ?? {})).toEqual(['showSelfComponents']);
	});
});

describe('rehydrating from disk', () => {
	it('reads a stored choice back', async () => {
		saved({ showSelfComponents: false });
		await useCatalogVisibilityStore.persist.rehydrate();

		expect(useCatalogVisibilityStore.getState().showSelfComponents).toBe(false);
	});

	it('leaves the action callable', async () => {
		saved({ showSelfComponents: false });
		await useCatalogVisibilityStore.persist.rehydrate();

		useCatalogVisibilityStore.getState().setShowSelfComponents(true);
		expect(useCatalogVisibilityStore.getState().showSelfComponents).toBe(true);
	});
});

describe('normaliseShowSelfComponents', () => {
	it('keeps a real boolean', () => {
		expect(normaliseShowSelfComponents(true)).toBe(true);
		expect(normaliseShowSelfComponents(false)).toBe(false);
	});

	it.each([['a string'], [null], [undefined], [0], [{ show: true }]])(
		'defaults to true (show) for %o, matching the opt-out default',
		(value) => {
			expect(normaliseShowSelfComponents(value)).toBe(true);
		}
	);
});
