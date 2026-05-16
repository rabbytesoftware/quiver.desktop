import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((_event, _handler) => Promise.resolve(() => {})),
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn().mockResolvedValue([]),
}));

const arrowStoreMocks = {
	resetArrows: vi.fn(),
	upsertArrow: vi.fn(),
	removeArrow: vi.fn(),
	applyRuntimeUpdate: vi.fn(),
	arrows: new Map<string, unknown>(),
};

vi.mock('../store/arrows', () => ({
	useArrowStore: {
		getState: () => arrowStoreMocks,
	},
}));

const statusStoreMocks = {
	setStatus: vi.fn(),
};

vi.mock('../store/status', () => ({
	useStatusStore: {
		getState: () => statusStoreMocks,
	},
}));

const connectionStoreMocks = {
	setFromEvent: vi.fn(),
};

vi.mock('@/lib/connection/store', () => ({
	useConnectionStore: {
		getState: () => connectionStoreMocks,
	},
}));

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { setupListeners } from './index';

beforeEach(() => {
	vi.clearAllMocks();
	arrowStoreMocks.arrows = new Map();
});

describe('setupListeners', () => {
	it('registers core://status, arrow://event, runtime://update, and connection://changed listeners', async () => {
		await setupListeners();
		const listenMock = listen as ReturnType<typeof vi.fn>;
		const channels = listenMock.mock.calls.map((args: unknown[]) => args[0] as string);
		expect(channels).toContain('core://status');
		expect(channels).toContain('arrow://event');
		expect(channels).toContain('runtime://update');
		expect(channels).toContain('connection://changed');
		expect(channels).not.toContain('arrow://hydrate');
	});

	it('calls get_arrows and get_connections on core://status ready', async () => {
		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'core://status') {
				handler({ payload: { status: 'ready' } });
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(invoke).toHaveBeenCalledWith('get_arrows');
		expect(invoke).toHaveBeenCalledWith('get_connections');
	});

	it('arrow://event with event=removed calls removeArrow', async () => {
		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'arrow://event') {
				handler({ payload: { event: 'removed', namespace: 'my/arrow' } });
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(arrowStoreMocks.removeArrow).toHaveBeenCalledWith('my/arrow');
		expect(arrowStoreMocks.upsertArrow).not.toHaveBeenCalled();
	});

	it('arrow://event with a non-removed event calls upsertArrow with merged fields', async () => {
		const existing = {
			namespace: 'my/arrow',
			name: 'Old Name',
			description: 'old desc',
			tags: [],
			icon: null,
			banner: null,
			version: '1.0.0',
			state: 'ready' as const,
			active_run: null,
			last_return: null,
		};
		arrowStoreMocks.arrows = new Map([['my/arrow', existing]]);

		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'arrow://event') {
				handler({
					payload: {
						event: 'added',
						namespace: 'my/arrow',
						name: 'New Name',
						description: 'new desc',
						tags: ['tag1'],
						icon: null,
						banner: null,
					},
				});
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(arrowStoreMocks.upsertArrow).toHaveBeenCalledWith({
			namespace: 'my/arrow',
			name: 'New Name',
			description: 'new desc',
			tags: ['tag1'],
			icon: null,
			banner: null,
			version: '1.0.0',
			state: 'ready',
			active_run: null,
			last_return: null,
		});
		expect(arrowStoreMocks.removeArrow).not.toHaveBeenCalled();
	});

	it('runtime://update handler calls applyRuntimeUpdate', async () => {
		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'runtime://update') {
				handler({
					payload: {
						namespace: 'my/arrow',
						state: 'running',
						active_run: null,
						last_return: null,
					},
				});
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(arrowStoreMocks.applyRuntimeUpdate).toHaveBeenCalledWith({
			namespace: 'my/arrow',
			state: 'running',
			active_run: null,
			last_return: null,
		});
	});

	it('connection://changed handler calls setFromEvent', async () => {
		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'connection://changed') {
				handler({
					payload: {
						connections: [
							{
								id: 'c1',
								name: 'Remote',
								kind: 'remote',
								url: 'http://localhost:7070',
								api_version: 'v0',
							},
						],
						active_id: 'c1',
					},
				});
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(connectionStoreMocks.setFromEvent).toHaveBeenCalledWith(
			[{ id: 'c1', name: 'Remote', kind: 'remote', url: 'http://localhost:7070', api_version: 'v0' }],
			'c1'
		);
	});

	it('core://status starting calls resetArrows', async () => {
		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'core://status') {
				handler({ payload: { status: 'starting' } });
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(arrowStoreMocks.resetArrows).toHaveBeenCalled();
		expect(invoke).not.toHaveBeenCalledWith('get_arrows');
	});

	it('core://status ready hydrates arrows from invoke response', async () => {
		(invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
			if (cmd === 'get_arrows') {
				return Promise.resolve([
					{
						namespace: 'hydrated/arrow',
						name: 'Hydrated',
						description: '',
						tags: [],
						icon: null,
						banner: null,
						versions: [{ ref: 'main', version: '0.1.0', state: 'ready' }],
					},
				]);
			}
			if (cmd === 'get_connections') {
				return Promise.resolve({ connections: [], active_id: 'local' });
			}
			return Promise.resolve([]);
		});

		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'core://status') {
				handler({ payload: { status: 'ready' } });
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(arrowStoreMocks.upsertArrow).toHaveBeenCalled();
	});

	it('arrow://event non-removed with no existing arrow uses default field values', async () => {
		// arrows map is empty — no existing entry
		arrowStoreMocks.arrows = new Map();

		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'arrow://event') {
				handler({
					payload: {
						event: 'updated',
						namespace: 'brand/new',
						// name, description, tags, icon, banner intentionally omitted to exercise defaults
					},
				});
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(arrowStoreMocks.upsertArrow).toHaveBeenCalledWith({
			namespace: 'brand/new',
			name: '',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '',
			state: 'ready',
			active_run: null,
			last_return: null,
		});
	});
});
