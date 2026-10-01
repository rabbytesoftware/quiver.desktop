import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import type { ArrowDetail, ArrowState } from '@/domain/arrow';
import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { useSelectorSwitchStore } from '@/features/arrow-details/stores/selector-switch-store';
import { useArrowStore } from '@/lib/core-store';
import { apiFetch } from '@/lib/transport/api';
import type { Backend } from '@/lib/transport/backend';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { untilUninstalled, useSelectorSwitch } from './use-selector-switch';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn() };
});
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

const FROM = 'github.com/char2cs/crowbar@stable';
const TO = 'github.com/char2cs/crowbar@beta';
const enc = encodeURIComponent;

function detail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return {
		namespace: FROM,
		name: 'crowbar',
		description: '',
		license: '',
		url: '',
		tags: [],
		media: { icon: null, banner: null },
		maintainers: [],
		credits: [],
		netbridge: [],
		variables: [],
		targets: [],
		state: 'ready',
		user_installed: true,
		selector: 'stable',
		selector_kind: 'channel',
		resolved_ref: 'v1.3.0',
		installed_commit: 'abc',
		available: null,
		outdated: false,
		active_run: null,
		last_return: null,
		channels: [],
		readme: null,
		dependencies: [],
		dependents: [],
		...overrides,
	};
}

function seedStore(namespace: string, state: ArrowState): void {
	useArrowStore.getState().reset();
	useArrowStore.getState().setCatalog([
		{
			connectionId: 'c',
			namespace,
			name: 'x',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '',
		},
	]);
	moveTo(namespace, state);
}

function moveTo(namespace: string, state: ArrowState): void {
	useArrowStore.getState().applyRuntimeUpdate({ namespace, state, active_run: null, last_return: null });
}

function calls(): string[] {
	return mockApiFetch.mock.calls.map(
		([path, init]) => `${(init as RequestInit | undefined)?.method ?? 'GET'} ${path}`
	);
}

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

const UNINSTALL = `POST /v0/runtime/${enc(FROM)}/uninstall`;
const DELETE = `DELETE /v0/arrow/${enc(FROM)}`;
const REGISTER = `POST /v0/arrow/${enc(TO)}`;
const INSTALL = `POST /v0/runtime/${enc(TO)}/install`;

/** Answers every call, failing the one whose `METHOD path` is `failing`. */
function failingAt(failing: string | null): void {
	mockApiFetch.mockImplementation((path: string, init?: RequestInit) => {
		const call = `${init?.method ?? 'GET'} ${path}`;
		return call === failing ? Promise.reject(new Error(`${call} refused`)) : Promise.resolve(undefined);
	});
}

function renderSwitch(d: ArrowDetail = detail(), values: Record<string, string> = {}) {
	const onRegistered = vi.fn();
	const view = renderHook(() => useSelectorSwitch(d, values, onRegistered), { wrapper });
	return { ...view, onRegistered };
}

/** Starts a switch to `beta` and walks the old row through a real uninstall once it is asked for. */
async function switchWithUninstall(result: { current: ReturnType<typeof useSelectorSwitch> }): Promise<void> {
	let done: Promise<void> = Promise.resolve();
	act(() => {
		done = result.current.switchTo('beta');
	});
	await waitFor(() => expect(calls()).toContain(UNINSTALL));
	act(() => moveTo(FROM, 'uninstalling'));
	act(() => moveTo(FROM, 'absent'));
	await act(() => done);
}

beforeEach(() => {
	mockApiFetch.mockReset();
	failingAt(null);
	useSelectorSwitchStore.setState({ active: null, failure: null });
});

afterEach(() => {
	resetBackend();
});

describe('useSelectorSwitch', () => {
	it('registers the new identity first and moves the page to it, then uninstalls, forgets and reinstalls', async () => {
		seedStore(FROM, 'ready');
		const refresh = vi.fn();
		useArrowStore.getState().setCatalogRefresh(refresh);
		const { result, onRegistered } = renderSwitch(detail(), { PORT: '1' });

		let done: Promise<void> = Promise.resolve();
		act(() => {
			done = result.current.switchTo('beta');
		});
		await waitFor(() => expect(calls()).toEqual([REGISTER, UNINSTALL]));
		expect(onRegistered).toHaveBeenCalledWith(TO);
		expect(result.current.pending).toBe(true);

		act(() => moveTo(FROM, 'uninstalling'));
		act(() => moveTo(FROM, 'absent'));
		await act(() => done);

		expect(calls()).toEqual([REGISTER, UNINSTALL, DELETE, INSTALL]);
		const install = mockApiFetch.mock.calls[3][1] as RequestInit;
		expect(JSON.parse(String(install.body))).toEqual({ variables: { PORT: '1' } });
		expect(result.current.pending).toBe(false);
		expect(result.current.failure).toBeNull();
		expect(result.current.registerError).toBeNull();
		expect(refresh).toHaveBeenCalled();
	});

	it('only swaps the catalog rows when nothing is on disk', async () => {
		seedStore(FROM, 'absent');
		const { result, onRegistered } = renderSwitch(detail({ state: 'absent' }));

		await act(() => result.current.switchTo('v1.2.0'));

		const pinned = 'github.com/char2cs/crowbar@v1.2.0';
		expect(calls()).toEqual([`POST /v0/arrow/${enc(pinned)}`, DELETE]);
		expect(onRegistered).toHaveBeenCalledWith(pinned);
	});

	it('does nothing for the selector the row already follows', async () => {
		const { result } = renderSwitch();
		await act(() => result.current.switchTo('stable'));
		expect(mockApiFetch).not.toHaveBeenCalled();
	});

	it('changes nothing and stays on the page when the new identity cannot be registered', async () => {
		seedStore(FROM, 'ready');
		failingAt(REGISTER);
		const { result, onRegistered } = renderSwitch();

		await act(() => result.current.switchTo('beta'));

		expect(calls()).toEqual([REGISTER]);
		expect(onRegistered).not.toHaveBeenCalled();
		expect(result.current.registerError).toBe(`${REGISTER} refused`);
		expect(result.current.failure).toBeNull();

		act(() => result.current.dismissRegisterError());
		expect(result.current.registerError).toBeNull();
	});

	it('reports a rejected uninstall: the new identity exists, the old one is still installed', async () => {
		seedStore(FROM, 'ready');
		failingAt(UNINSTALL);
		const { result, onRegistered } = renderSwitch();

		await act(() => result.current.switchTo('beta'));

		expect(calls()).toEqual([REGISTER, UNINSTALL]);
		expect(onRegistered).toHaveBeenCalledWith(TO);
		expect(result.current.failure).toEqual({
			step: 'uninstall',
			from: FROM,
			to: TO,
			reason: `${UNINSTALL} refused`,
		});
	});

	it('reports an uninstall that settles anywhere but not installed, and forgets nothing', async () => {
		seedStore(FROM, 'ready');
		const { result } = renderSwitch();

		let done: Promise<void> = Promise.resolve();
		act(() => {
			done = result.current.switchTo('beta');
		});
		await waitFor(() => expect(calls()).toContain(UNINSTALL));
		act(() => moveTo(FROM, 'uninstalling'));
		act(() => moveTo(FROM, 'ready'));
		await act(() => done);

		expect(calls()).toEqual([REGISTER, UNINSTALL]);
		expect(result.current.failure?.step).toBe('uninstall');

		act(() => result.current.dismissFailure());
		expect(result.current.failure).toBeNull();
	});

	it('reports a failed forget: the old row is uninstalled but still in the library, nothing new installed', async () => {
		seedStore(FROM, 'ready');
		failingAt(DELETE);
		const { result } = renderSwitch();

		await switchWithUninstall(result);

		expect(calls()).toEqual([REGISTER, UNINSTALL, DELETE]);
		expect(result.current.failure).toMatchObject({ step: 'remove', from: FROM, to: TO });
	});

	it('reports a failed install: the old row is gone and the new one is in the library, not installed', async () => {
		seedStore(FROM, 'ready');
		failingAt(INSTALL);
		const { result } = renderSwitch();

		await switchWithUninstall(result);

		expect(calls()).toEqual([REGISTER, UNINSTALL, DELETE, INSTALL]);
		expect(result.current.failure).toMatchObject({ step: 'install', from: FROM, to: TO });
	});

	it('installs Quiver’s own new row with the release asset of the ref that new row resolved to', async () => {
		const from = `${QUIVER_DESKTOP_NAMESPACE}@stable`;
		const to = `${QUIVER_DESKTOP_NAMESPACE}@nightly-latest`;
		seedStore(from, 'ready');
		const resolveReleaseAsset = vi.fn().mockResolvedValue({
			tag: 'nightly-latest',
			name: 'q.AppImage',
			url: 'https://x/q.AppImage',
			checksum: 'c'.repeat(64),
		});
		installBackend({ resolveReleaseAsset } as unknown as Backend);
		mockApiFetch.mockImplementation((_path: string, init?: RequestInit) =>
			Promise.resolve(
				init?.method === undefined
					? { namespace: to, resolved_ref: 'nightly-latest', available: { ref: 'ignored', commit: '' } }
					: undefined
			)
		);
		const { result } = renderSwitch(detail({ namespace: from, selector: 'stable' }));

		let done: Promise<void> = Promise.resolve();
		act(() => {
			done = result.current.switchTo('nightly-latest');
		});
		await waitFor(() => expect(calls()).toContain(`POST /v0/runtime/${enc(from)}/uninstall`));
		act(() => moveTo(from, 'absent'));
		await act(() => done);

		expect(resolveReleaseAsset).toHaveBeenCalledWith('nightly-latest');
		const [, install] = mockApiFetch.mock.calls[mockApiFetch.mock.calls.length - 1];
		expect(JSON.parse(String((install as RequestInit).body))).toEqual({
			variables: { QUIVER_RELEASE_ASSET_URL: 'https://x/q.AppImage', QUIVER_RELEASE_CHECKSUM: 'c'.repeat(64) },
		});
		expect(calls()).toContain(`GET /v0/arrow/${enc(to)}`);
	});
});

describe('untilUninstalled', () => {
	beforeEach(() => seedStore(FROM, 'ready'));

	it('settles at once when a check finds the row already not installed', async () => {
		const { settled, check } = untilUninstalled(FROM, 1000);
		act(() => moveTo(FROM, 'removed'));
		check();
		await expect(settled).resolves.toBeUndefined();
	});

	it('settles on check for a row a no-op uninstall left absent without ever reporting uninstalling', async () => {
		useArrowStore.getState().reset();
		useArrowStore.getState().setCatalog([
			{
				connectionId: 'c',
				namespace: FROM,
				name: 'x',
				description: '',
				tags: [],
				icon: null,
				banner: null,
				version: '',
			},
		]);
		const { settled, check } = untilUninstalled(FROM, 1000);
		check();
		await expect(settled).resolves.toBeUndefined();
	});

	it('gives up after its timeout instead of waiting forever', async () => {
		const { settled } = untilUninstalled(FROM, 10);
		await expect(settled).rejects.toThrow(/not uninstalled within/);
	});

	it('ignores a namespace the store does not know', async () => {
		const { settled, check, cancel } = untilUninstalled('github.com/x/y@z', 10);
		check();
		await expect(settled).rejects.toThrow(/not uninstalled within/);
		cancel();
	});
});
