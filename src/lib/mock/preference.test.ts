import { beforeEach, describe, expect, it, vi } from 'vitest';

import { mockForcedByEnv, readMockPreference } from './preference';
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

	it('falls back to normal for a scenario that no longer exists', () => {
		write({ enabled: true, scenario: 'rabbyte-only-2024' });
		expect(readMockPreference()).toEqual({ enabled: true, scenario: 'normal' });
	});

	it('falls back to normal when the scenario is not even a string', () => {
		write({ enabled: true, scenario: 7 });
		expect(readMockPreference().scenario).toBe('normal');
	});
});

describe('VITE_QUIVER_MOCK', () => {
	it('forces the mock on even with nothing persisted', () => {
		vi.stubEnv('VITE_QUIVER_MOCK', '1');
		expect(mockForcedByEnv()).toBe(true);
		expect(readMockPreference()).toEqual({ enabled: true, scenario: 'normal' });
		vi.unstubAllEnvs();
	});

	it('accepts "true" as well as "1", and nothing else', () => {
		vi.stubEnv('VITE_QUIVER_MOCK', 'true');
		expect(mockForcedByEnv()).toBe(true);
		vi.stubEnv('VITE_QUIVER_MOCK', '0');
		expect(mockForcedByEnv()).toBe(false);
		vi.unstubAllEnvs();
	});

	it('beats a persisted enabled:false', () => {
		write({ enabled: false, scenario: 'empty' });
		vi.stubEnv('VITE_QUIVER_MOCK', '1');
		expect(readMockPreference().enabled).toBe(true);
		vi.unstubAllEnvs();
	});

	it('takes the scenario from VITE_QUIVER_SCENARIO', () => {
		vi.stubEnv('VITE_QUIVER_MOCK', '1');
		vi.stubEnv('VITE_QUIVER_SCENARIO', 'extreme');
		expect(readMockPreference().scenario).toBe('extreme');
		vi.unstubAllEnvs();
	});

	it('falls back to normal for a scenario name it does not know', () => {
		vi.stubEnv('VITE_QUIVER_MOCK', '1');
		vi.stubEnv('VITE_QUIVER_SCENARIO', 'nonsense');
		expect(readMockPreference().scenario).toBe('normal');
		vi.unstubAllEnvs();
	});

	it('is off when unset', () => {
		expect(mockForcedByEnv()).toBe(false);
	});
});
