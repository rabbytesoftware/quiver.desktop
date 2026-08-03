import { beforeEach, describe, expect, it, vi } from 'vitest';

import { readMockPreference } from './preference';
import { MOCK_STORAGE_KEY } from './store';

function write(state: unknown): void {
	localStorage.setItem(MOCK_STORAGE_KEY, JSON.stringify({ state, version: 0 }));
}

beforeEach(() => {
	localStorage.clear();
	vi.restoreAllMocks();
});

describe('readMockPreference', () => {
	it('is off when nothing has ever been written', () => {
		expect(readMockPreference()).toEqual({ enabled: false, scenario: 'normal' });
	});

	it('reads the scenario the store persisted', () => {
		write({ enabled: true, scenario: 'extreme' });
		expect(readMockPreference()).toEqual({ enabled: true, scenario: 'extreme' });
	});

	it('is off when the flag is false, whatever the scenario says', () => {
		write({ enabled: false, scenario: 'extreme' });
		expect(readMockPreference().enabled).toBe(false);
	});

	// Every failure mode lands on OFF. This ships in release builds, where
	// someone with a corrupt value may not know the flag exists and cannot reach
	// the setting that would clear it — so a bad read must never be able to keep
	// them out of their real library.
	it('is off when the stored value is not JSON', () => {
		localStorage.setItem(MOCK_STORAGE_KEY, '{not json');
		expect(readMockPreference().enabled).toBe(false);
	});

	it('is off when the payload has no state object', () => {
		localStorage.setItem(MOCK_STORAGE_KEY, JSON.stringify({ version: 0 }));
		expect(readMockPreference().enabled).toBe(false);
	});

	it('is off when state is null rather than an object', () => {
		localStorage.setItem(MOCK_STORAGE_KEY, JSON.stringify({ state: null }));
		expect(readMockPreference().enabled).toBe(false);
	});

	it('is off when localStorage itself throws', () => {
		vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
			throw new Error('private mode');
		});
		expect(readMockPreference().enabled).toBe(false);
	});

	// A scenario that no longer exists must not brick the boot — falls back
	// rather than throwing, same reasoning as `getScenario` one layer down.
	it('falls back to normal for a scenario that no longer exists', () => {
		write({ enabled: true, scenario: 'rabbyte-only-2024' });
		expect(readMockPreference()).toEqual({ enabled: true, scenario: 'normal' });
	});

	it('falls back to normal when the scenario is not even a string', () => {
		write({ enabled: true, scenario: 7 });
		expect(readMockPreference().scenario).toBe('normal');
	});
});
