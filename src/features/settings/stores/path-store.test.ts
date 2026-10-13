import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import * as pathApi from '@/features/settings/api/path-api';
import { disposeMock, installMock } from '@/lib/mock';
import { useMockStore } from '@/lib/mock/store';
import { ApiError } from '@/lib/transport/api';
import { resetBackend } from '@/lib/transport/backend';

import { usePathStore } from './path-store';

beforeEach(() => {
	installMock('normal');
	useMockStore.getState().resetFaults();
	usePathStore.setState({
		status: null,
		loading: true,
		settingUp: false,
		unavailable: false,
		error: null,
		setupError: null,
	});
});

afterEach(() => {
	vi.restoreAllMocks();
	disposeMock();
	resetBackend();
});

describe('the PATH store', () => {
	it('loads a PATH that is not configured', async () => {
		await usePathStore.getState().load();
		const s = usePathStore.getState();
		expect(s.loading).toBe(false);
		expect(s.status).toMatchObject({ onPath: false, configured: false, binDir: '/home/mock/.quiver/bin' });
		expect(s.status?.files.length).toBeGreaterThan(0);
	});

	it('loads a PATH that is already configured', async () => {
		await pathApi.setupPath();
		await usePathStore.getState().load();
		expect(usePathStore.getState().status?.configured).toBe(true);
	});

	it('sets up PATH and adopts the returned status', async () => {
		await usePathStore.getState().load();
		await usePathStore.getState().setup();
		const s = usePathStore.getState();
		expect(s.status?.configured).toBe(true);
		expect(s.settingUp).toBe(false);
		expect(s.setupError).toBeNull();
	});

	it('keeps the status and records a setup failure separately', async () => {
		await usePathStore.getState().load();
		useMockStore.getState().setFault('path', 100);
		await usePathStore.getState().setup();
		const s = usePathStore.getState();
		expect(s.setupError).not.toBeNull();
		expect(s.error).toBeNull();
		expect(s.status?.configured).toBe(false);
		expect(s.settingUp).toBe(false);
	});

	it('clears a stale setup error when loaded again', async () => {
		usePathStore.setState({ setupError: 'old' });
		await usePathStore.getState().load();
		expect(usePathStore.getState().setupError).toBeNull();
	});

	it('records a load failure', async () => {
		useMockStore.getState().setFault('path', 100);
		await usePathStore.getState().load();
		expect(usePathStore.getState().error).not.toBeNull();
		expect(usePathStore.getState().unavailable).toBe(false);
	});

	it('marks the entry unavailable on a core without the endpoint', async () => {
		vi.spyOn(pathApi, 'getPathStatus').mockRejectedValueOnce(new ApiError('not found', 404));
		await usePathStore.getState().load();
		const s = usePathStore.getState();
		expect(s.unavailable).toBe(true);
		expect(s.error).toBeNull();
		expect(s.loading).toBe(false);
	});

	it('stringifies a non-Error rejection from a load and from a setup', async () => {
		vi.spyOn(pathApi, 'getPathStatus').mockRejectedValueOnce('boom');
		await usePathStore.getState().load();
		expect(usePathStore.getState().error).toBe('boom');

		vi.spyOn(pathApi, 'setupPath').mockRejectedValueOnce('nope');
		await usePathStore.getState().setup();
		expect(usePathStore.getState().setupError).toBe('nope');
	});
});
