import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

import { getConsoleController } from './console-controller';
import { useConsoleStore } from './console-store';

const mockFetch = vi.mocked(apiFetch);

beforeEach(() => {
	mockFetch.mockReset();
	useConsoleStore.setState(useConsoleStore.getInitialState(), true);
});

describe("the app's console controller", () => {
	it('is created once', () => {
		expect(getConsoleController()).toBe(getConsoleController());
	});

	it("reads the daemon's capability and command table over the API once it is ready", async () => {
		mockFetch.mockImplementation(async (path: string) =>
			path === '/versions'
				? { version: 'nightly-latest', features: ['console.v1'] }
				: { commands: [{ path: ['install'], short: 'install an arrow', usage: 'install <ns>' }] }
		);

		getConsoleController().sync('local', true);
		await vi.waitFor(() => expect(useConsoleStore.getState().commandsLoaded).toBe(true));

		expect(mockFetch).toHaveBeenCalledWith('/versions');
		expect(mockFetch).toHaveBeenCalledWith('/v0/console/commands');
		expect(useConsoleStore.getState().support).toBe('supported');
		expect(useConsoleStore.getState().commands[0].path).toEqual(['install']);
	});
});
