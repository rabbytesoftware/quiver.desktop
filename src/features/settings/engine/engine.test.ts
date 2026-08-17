import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { currentMock, disposeMock, installMock } from '@/lib/mock';
import { setMockCorrected } from '@/lib/mock/server/handlers/config';
import { useMockStore } from '@/lib/mock/store';
import { resetBackend } from '@/lib/transport/backend';

import * as engineApi from './api';
import { useEngineStore } from './store';

beforeEach(() => {
	installMock('normal');
	useMockStore.getState().resetFaults();
	useMockStore.getState().resetChaos();
	useEngineStore.setState({ view: null, rejected: [], loading: false, error: null });
});

afterEach(() => {
	vi.restoreAllMocks();
	disposeMock();
	resetBackend();
});

describe('the engine config store', () => {
	it('loads the three views and the restart list', async () => {
		await useEngineStore.getState().load();
		const view = useEngineStore.getState().view;
		expect(view?.configured.netbridge.ephemeral_port_start).toBe(49152);
		expect(view?.defaults.logger.level).toBe('info');
		expect(view?.restart_required).toEqual([]);
	});

	it('persists a change and reports it as needing a restart', async () => {
		await useEngineStore.getState().load();
		await useEngineStore.getState().patch({ netbridge: { ephemeral_port_start: 27015 } });
		const view = useEngineStore.getState().view;
		expect(view?.configured.netbridge.ephemeral_port_start).toBe(27015);
		expect(view?.restart_required).toContain('netbridge.ephemeral_port_start');
		expect(useEngineStore.getState().rejected).toEqual([]);
	});

	it('restores a default when the patch sends null', async () => {
		await useEngineStore.getState().load();
		await useEngineStore.getState().patch({ logger: { level: 'debug' } });
		await useEngineStore.getState().patch({ logger: { level: null } });
		expect(useEngineStore.getState().view?.configured.logger.level).toBe('info');
	});

	it('keeps the settings that applied and surfaces the ones that did not', async () => {
		await useEngineStore.getState().load();
		await useEngineStore.getState().patch({
			logger: { level: 'nonsense' },
			netbridge: { ephemeral_port_start: 30000 },
		});
		expect(useEngineStore.getState().rejected.map((r) => r.key)).toEqual(['logger.level']);
		expect(useEngineStore.getState().view?.configured.netbridge.ephemeral_port_start).toBe(30000);
	});

	it('round-trips sections the UI never renders', async () => {
		await useEngineStore.getState().load();
		await useEngineStore.getState().patch({ logger: { level: 'warn' } });
		expect(useEngineStore.getState().view?.configured.search).toBeDefined();
	});

	it('records an error when the daemon cannot be reached', async () => {
		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 100 } });
		await useEngineStore.getState().load();
		expect(useEngineStore.getState().error).not.toBeNull();
	});

	it('surfaces settings the daemon already corrected on disk', async () => {
		setMockCorrected(currentMock()!.world, ['logger.level']);
		await useEngineStore.getState().load();
		expect(useEngineStore.getState().view?.corrected).toEqual([
			{ key: 'logger.level', message: 'unusable value, default applied' },
		]);
	});

	it('populates rejection reasons without setting an error when nothing at all applied', async () => {
		await useEngineStore.getState().load();
		await useEngineStore.getState().patch({ logger: { level: 'nonsense' } });
		expect(useEngineStore.getState().rejected).toEqual([{ key: 'logger.level', message: 'unusable log level' }]);
		expect(useEngineStore.getState().error).toBeNull();
	});

	it('treats api.host as read-only whether given a value or a null', async () => {
		await useEngineStore.getState().load();
		await useEngineStore.getState().patch({ api: { host: null } });
		expect(useEngineStore.getState().rejected).toEqual([{ key: 'api.host', message: 'read-only' }]);
	});

	it('records an error when a patch cannot reach the daemon', async () => {
		await useEngineStore.getState().load();
		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 100 } });
		await useEngineStore.getState().patch({ logger: { level: 'warn' } });
		expect(useEngineStore.getState().error).not.toBeNull();
	});

	it('stringifies a non-Error rejection from a load', async () => {
		vi.spyOn(engineApi, 'getConfig').mockRejectedValueOnce('boom');
		await useEngineStore.getState().load();
		expect(useEngineStore.getState().error).toBe('boom');
	});

	it('stringifies a non-Error rejection from a patch', async () => {
		await useEngineStore.getState().load();
		vi.spyOn(engineApi, 'patchConfig').mockRejectedValueOnce('nope');
		await useEngineStore.getState().patch({ logger: { level: 'warn' } });
		expect(useEngineStore.getState().error).toBe('nope');
	});

	it('falls back to the status line when a patch response is not JSON at all', async () => {
		await useEngineStore.getState().load();
		useMockStore.setState({ unreachable: true });
		await useEngineStore.getState().patch({ logger: { level: 'warn' } });
		expect(useEngineStore.getState().error).toContain('502');
	});
});
