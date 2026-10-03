import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';
import type { ArrowDetail, ArrowLifecycle, ArrowTarget } from '@/domain/arrow';
import type { ResolvedReleaseAsset } from '@/domain/release';
import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { useSelectorSwitchStore } from '@/features/arrow-details/stores/selector-switch-store';
import { useArrowStore } from '@/lib/core-store';
import { apiFetch, ApiError } from '@/lib/transport/api';
import type { Backend } from '@/lib/transport/backend';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { Hero } from './hero';

// `apiRequest` (the update call, which needs the response status) answers
// through the same `apiFetch` mock, so every call is asserted in one place;
// `update.status` is the status it reports -- 202 unless a test says otherwise.
const update = vi.hoisted(() => ({ status: 202 }));
vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	const apiFetch = vi.fn();
	return {
		...actual,
		apiFetch,
		apiRequest: vi.fn(async (path: string, init?: RequestInit) => ({
			status: update.status,
			data: await apiFetch(path, init),
		})),
	};
});
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

const PLATFORM = 'darwin/arm64';

const LIFECYCLE: ArrowLifecycle = {
	install: [runStep('Fetch archive')],
	update: [runStep('Fetch new version')],
	execute: [runStep('Start process')],
	stop: [signalStep('Signal process')],
	uninstall: [runStep('Remove workdir')],
};

const TARGET: ArrowTarget = {
	platform: PLATFORM,
	requirement: { cpu_cores: 1, memory_gb: 1, disk_gb: 1 },
	lifecycle: LIFECYCLE,
	methods: [],
};

function detail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return {
		namespace: 'github.com/rabbyte/minecraft@v1.21.4',
		name: 'Minecraft Server',
		description: 'A vanilla Minecraft Java Edition server.',
		license: 'MIT',
		url: 'https://github.com/rabbyte/minecraft',
		tags: ['game', 'server'],
		media: { icon: null, banner: null },
		maintainers: [],
		credits: [],
		netbridge: [],
		variables: [{ name: 'server-name', description: 'Shown in the list.', type: 'string', default: 'My Server' }],
		targets: [TARGET],
		state: 'ready',
		user_installed: true,
		selector: 'v1.21.4',
		selector_kind: 'pin',
		resolved_ref: 'v1.21.4',
		installed_commit: '',
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

function wrapper(
	client: QueryClient = new QueryClient({
		defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
	})
) {
	return function Wrapper({ children }: { children: React.ReactNode }) {
		return createElement(QueryClientProvider, { client }, children);
	};
}

function renderHero(props: Partial<React.ComponentProps<typeof Hero>> = {}) {
	const onValueChange = vi.fn();
	render(
		<Hero
			channelsLoading={false}
			detail={detail()}
			onValueChange={onValueChange}
			platform={PLATFORM}
			values={{}}
			{...props}
		/>,
		{
			wrapper: wrapper(),
		}
	);
	return { onValueChange };
}

/** The last `apiFetch` call, so a test can read what was actually sent. */
function lastCall(): [string, RequestInit] {
	const calls = mockApiFetch.mock.calls;
	return calls[calls.length - 1] as unknown as [string, RequestInit];
}

function bodyOf([, init]: [string, RequestInit]): unknown {
	return JSON.parse(String(init.body));
}

/** Quiver's own row: following `stable`, installed at `stable-1.0`, with `stable-1.1` ahead of it. */
function selfDetail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return detail({
		namespace: `${QUIVER_DESKTOP_NAMESPACE}@stable`,
		name: 'Quiver',
		variables: [
			{ name: 'QUIVER_RELEASE_ASSET_URL', description: 'Download URL.', type: 'string', default: '' },
			{ name: 'QUIVER_RELEASE_CHECKSUM', description: 'SHA-256.', type: 'string', default: '' },
		],
		selector: 'stable',
		selector_kind: 'channel',
		resolved_ref: 'stable-1.0',
		installed_commit: 'abc',
		available: { ref: 'stable-1.1', commit: 'def' },
		outdated: true,
		...overrides,
	});
}

const RESOLVED: ResolvedReleaseAsset = {
	tag: 'stable-1.1',
	name: 'quiver-desktop_0.1.0_amd64.AppImage',
	url: 'https://github.com/rabbytesoftware/quiver.desktop/releases/download/stable-1.1/quiver-desktop_0.1.0_amd64.AppImage',
	checksum: 'c'.repeat(64),
};

/** Stands in for the native resolver behind `backend()`. */
function resolverAnswering(result: ResolvedReleaseAsset | { reject: unknown }) {
	const resolveReleaseAsset =
		'reject' in result ? vi.fn().mockRejectedValue(result.reject) : vi.fn().mockResolvedValue(result);
	installBackend({ resolveReleaseAsset, getPlatform: () => Promise.resolve(PLATFORM) } as unknown as Backend);
	return resolveReleaseAsset;
}

/**
 * The real `backend()` is a Tauri `invoke` bridge with nothing to answer it
 * in jsdom, so `hero.tsx`'s `resolveRealPlatform()` (used to gate an
 * add-to-library click) falls back to the UA guess -- an unmocked value this
 * environment doesn't control. Every test installs this default so that
 * gate resolves to the same `PLATFORM` the `platform` prop already defaults
 * to; a test asserting the "genuinely unsupported" path overrides it (or the
 * `platform` prop, or both) explicitly instead of relying on that fallback.
 */
function platformBackend(platform: string = PLATFORM) {
	installBackend({ getPlatform: () => Promise.resolve(platform) } as unknown as Backend);
}

beforeEach(() => {
	mockApiFetch.mockReset();
	mockApiFetch.mockResolvedValue(undefined);
	update.status = 202;
	useSelectorSwitchStore.setState({ active: null, failure: null });
	useArrowStore.getState().setCatalogRefresh(() => {});
	platformBackend();
});

afterEach(() => {
	resetBackend();
});

describe('Hero', () => {
	it('renders identity: name, namespace, and status', () => {
		renderHero();
		expect(screen.getByRole('heading', { name: 'Minecraft Server' })).toBeInTheDocument();
		expect(screen.getByText('github.com/rabbyte/minecraft@v1.21.4')).toBeInTheDocument();
		expect(screen.getByText('Ready')).toBeInTheDocument();
	});

	it('shows the flicker spinner (not the static status icon) for a busy state', () => {
		renderHero({ detail: detail({ state: 'installing' }) });
		// "Installing…" appears twice (the status badge and the busy action
		// button) -- both are legitimate; this test only cares that the busy
		// treatment (the flicker spinner) is present at all.
		expect(screen.getAllByText('Installing…').length).toBeGreaterThan(0);
		expect(document.querySelector('[data-slot="flicker-spinner"]')).toBeInTheDocument();
	});

	it('shows "Not in library" and only the Add to Library action when user_installed is false', () => {
		renderHero({ detail: detail({ user_installed: false }) });
		expect(screen.getByText('Not in library')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Add to Library' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Start' })).not.toBeInTheDocument();
	});

	it('invalidates the arrow-detail query cache after Add to Library succeeds -- user_installed is not part of the live WS overlay, so only a refetch picks up the change', async () => {
		const user = userEvent.setup();
		const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
		const invalidateSpy = vi.spyOn(qc, 'invalidateQueries');
		render(
			<Hero
				channelsLoading={false}
				detail={detail({ user_installed: false })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper(qc) }
		);

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));

		await waitFor(() => expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['arrow'] }));
	});

	it('invalidates the arrow-detail query cache after Remove from Library succeeds too', async () => {
		const user = userEvent.setup();
		const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
		const invalidateSpy = vi.spyOn(qc, 'invalidateQueries');
		render(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'absent' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{
				wrapper: wrapper(qc),
			}
		);

		await user.click(screen.getByRole('button', { name: 'Remove from Library' }));

		await waitFor(() => expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['arrow'] }));
	});

	it('renders no tags row at all when there are no tags and nothing to flag', () => {
		renderHero({ detail: detail({ tags: [] }) });
		expect(document.querySelector('.mt-3.flex.flex-wrap')).not.toBeInTheDocument();
	});

	it('renders every tag', () => {
		renderHero();
		expect(screen.getByText('game')).toBeInTheDocument();
		expect(screen.getByText('server')).toBeInTheDocument();
	});

	it('renders the description', () => {
		renderHero();
		expect(screen.getByText('A vanilla Minecraft Java Edition server.')).toBeInTheDocument();
	});

	it('renders the license, with no channel or version picker when there are no channels to show', () => {
		renderHero();
		expect(screen.getByText('MIT')).toBeInTheDocument();
		expect(screen.queryByRole('combobox', { name: 'Channel' })).not.toBeInTheDocument();
		expect(screen.queryByRole('combobox', { name: 'Version' })).not.toBeInTheDocument();
	});

	it('renders no banner element when media.banner is absent', () => {
		renderHero();
		expect(document.querySelector('.aspect-2\\/1')).not.toBeInTheDocument();
	});

	it('renders a banner when media.banner is present', () => {
		renderHero({ detail: detail({ media: { icon: null, banner: 'https://example.com/banner.png' } }) });
		expect(document.querySelector('.aspect-2\\/1')).toBeInTheDocument();
	});

	it('shows no problem chip for a healthy ready arrow', () => {
		renderHero();
		expect(screen.queryByText('Issue')).not.toBeInTheDocument();
	});

	it('shows a problem chip for a detached arrow, opening a modal with the detached explanation', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'detached' }) });

		const chip = screen.getByRole('button', { name: /Issue/ });
		await user.click(chip);
		expect(await screen.findByText(/lost track of this process/)).toBeInTheDocument();
	});

	it('shows a problem chip for a failed last run, with the failed step’s own error text', async () => {
		const user = userEvent.setup();
		renderHero({
			detail: detail({
				state: 'ready',
				last_return: {
					method: 'install',
					outcome: 'failed',
					variables: {},
					steps: [
						{
							index: 0,
							title: 'Verify checksum',
							status: 'failed',
							type: 'run',
							error: 'checksum mismatch',
						},
					],
				},
			}),
		});

		await user.click(screen.getByRole('button', { name: /Issue/ }));
		expect(await screen.findByText('checksum mismatch')).toBeInTheDocument();
	});

	it('falls back to a generic failure message when the failed run carries no step-level error', async () => {
		const user = userEvent.setup();
		renderHero({
			detail: detail({
				state: 'ready',
				last_return: { method: 'install', outcome: 'failed', variables: {}, steps: [] },
			}),
		});

		await user.click(screen.getByRole('button', { name: /Issue/ }));
		expect(await screen.findByText('The last run did not finish successfully.')).toBeInTheDocument();
	});

	it('invokes install with the current variable values when Install is clicked', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'absent' }), values: { 'server-name': 'Custom' } });

		await user.click(screen.getByRole('button', { name: 'Install' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(
				expect.stringContaining('/install'),
				expect.objectContaining({ body: JSON.stringify({ variables: { 'server-name': 'Custom' } }) })
			)
		);
	});

	it('invokes execute (not a custom method name) when Start is clicked', async () => {
		const user = userEvent.setup();
		renderHero();

		await user.click(screen.getByRole('button', { name: 'Start' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.anything())
		);
	});

	it('invokes registerArrow for Add to Library, with no variables body concern', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ user_installed: false }) });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(
				expect.stringContaining(encodeURIComponent(detail().namespace)),
				expect.objectContaining({ method: 'POST' })
			)
		);
	});

	it('registers the identity it shows, with no JSON body, when the arrow has no channels to choose from', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ user_installed: false, channels: [] }) });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalled());
		expect(lastCall()[1]).toEqual({ method: 'POST' });
	});

	it('sequences Restart as stop then, once the live state reaches ready, execute -- not immediately after stop resolves', async () => {
		const user = userEvent.setup();
		const running = detail({ state: 'running' });
		const { rerender } = render(
			<Hero channelsLoading={false} detail={running} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />,
			{
				wrapper: wrapper(),
			}
		);

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/stop'), expect.anything()));

		// Only `stop` has fired -- `execute` must not fire until the parent
		// re-renders Hero with the live state actually at `ready`.
		expect(apiFetch).not.toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.anything());

		rerender(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>
		);

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.anything())
		);
	});

	it('does not fire the restart follow-up when the arrow reaches ready without a restart in flight', async () => {
		const { rerender } = render(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'running' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper() }
		);
		rerender(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>
		);
		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('invokes removeArrow when Remove from Library is clicked', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'absent' }) });

		await user.click(screen.getByRole('button', { name: 'Remove from Library' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining(encodeURIComponent(detail().namespace)), {
				method: 'DELETE',
			})
		);
	});

	it('invokes uninstall when Uninstall is clicked', async () => {
		const user = userEvent.setup();
		renderHero();

		await user.click(screen.getByRole('button', { name: 'Uninstall' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/uninstall'), expect.anything())
		);
	});

	it('invokes update when Update is clicked', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'outdated' }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/update'), expect.anything())
		);
	});

	it('sends no release variables when updating an ordinary arrow', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'outdated' }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalled());
		// core requires only the variables the method's own steps expand, and
		// a third-party arrow's update expands none of Quiver's. Sending them
		// anyway would be handing an unrelated arrow this app's download URL.
		expect(bodyOf(lastCall())).toEqual({ variables: {} });
	});

	it('invokes stop when Stop is clicked', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'running' }) });

		await user.click(screen.getByRole('button', { name: 'Stop' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/stop'), expect.anything()));
	});

	it('invokes uninstall for Reinstall too, sharing the same install call as a fresh install', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'removed' }) });

		await user.click(screen.getByRole('button', { name: 'Reinstall' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/install'), expect.anything())
		);
	});

	it('clears pendingKind and does not leave the button stuck disabled when a mutation rejects', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('offline'));
		const user = userEvent.setup();
		renderHero();

		const main = screen.getByRole('button', { name: 'Uninstall' });
		await user.click(main);
		await waitFor(() => expect(main).not.toBeDisabled());
	});

	// The bug this guards against: the backend already computes a precise,
	// structured reason for a failed action (here, the real shape core sends
	// back for `ErrPlatformNotSupported`) and the click handler used to throw
	// it away with a bare `catch { ... }`, leaving the button silently revert
	// to normal with no indication anything went wrong.
	it('surfaces the backend error message in a dialog when install rejects with an ApiError', async () => {
		mockApiFetch.mockRejectedValueOnce(new ApiError('no target for the current platform', 422));
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'absent' }) });

		await user.click(screen.getByRole('button', { name: 'Install' }));

		expect(await screen.findByRole('dialog')).toBeInTheDocument();
		expect(screen.getByText('no target for the current platform')).toBeInTheDocument();
	});

	// Not install-specific: every action that goes through this same catch
	// block must surface its own mutation's failure the same way.
	it('surfaces the backend error message for other action kinds too, e.g. Uninstall', async () => {
		mockApiFetch.mockRejectedValueOnce(new ApiError('arrow has dependents', 409));
		const user = userEvent.setup();
		renderHero();

		await user.click(screen.getByRole('button', { name: 'Uninstall' }));

		expect(await screen.findByRole('dialog')).toBeInTheDocument();
		expect(screen.getByText('arrow has dependents')).toBeInTheDocument();
	});

	it('dismisses the action-error dialog and allows retrying', async () => {
		mockApiFetch.mockRejectedValueOnce(new ApiError('no target for the current platform', 422));
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'absent' }) });

		await user.click(screen.getByRole('button', { name: 'Install' }));
		expect(await screen.findByRole('dialog')).toBeInTheDocument();

		await user.keyboard('{Escape}');
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());

		mockApiFetch.mockResolvedValueOnce(undefined);
		await user.click(screen.getByRole('button', { name: 'Install' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/install'), expect.anything())
		);
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('clears the restart-in-flight flag (not just pendingKind) when the stop leg of a restart itself rejects', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('offline'));
		const user = userEvent.setup();
		const { rerender } = render(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'running' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper() }
		);

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		// The rejection now also opens the action-error dialog, which marks the
		// rest of the page inert while it's up -- dismiss it to get back to a
		// state where the Restart button is reachable again, same as a real user
		// would after reading the message.
		expect(await screen.findByRole('dialog')).toBeInTheDocument();
		await user.keyboard('{Escape}');
		await waitFor(() => expect(screen.getByRole('button', { name: 'Restart' })).not.toBeDisabled());

		// If the failed restart's flag were left set, this transition to ready
		// would wrongly fire `execute` on its own.
		mockApiFetch.mockClear();
		rerender(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>
		);
		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('clears pendingKind and surfaces an error dialog when restart’s second leg (execute, once ready) itself rejects', async () => {
		const user = userEvent.setup();
		const { rerender } = render(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'running' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper() }
		);

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/stop'), expect.anything()));

		// The arrow reaching `ready` is what triggers the execute leg -- once
		// there, `computeActions` for `ready` no longer even offers a "Restart"
		// button (only `running`/`stopping`/`draining` do), so the real
		// assertion is that pendingKind was still cleared despite the
		// rejection: the newly-shown "Start" action must not be stuck disabled.
		// This is the same class of bug the action-error dialog exists to
		// close -- restart's second leg has its own separate catch, so it
		// must be asserted here too, not assumed covered by the first leg's
		// own test.
		mockApiFetch.mockRejectedValueOnce(new ApiError('no target for the current platform', 422));
		rerender(
			<Hero
				channelsLoading={false}
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>
		);

		await waitFor(() => expect(screen.getByRole('button', { name: 'Start' })).not.toBeDisabled());
		expect(await screen.findByRole('dialog')).toHaveTextContent('no target for the current platform');
		await user.keyboard('{Escape}');
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});
});

/**
 * The Channel/Version pair picks what a NEW library entry follows -- the
 * selector in the identity Add to Library registers. Once the arrow is in
 * the library, its selector is fixed: the Hero shows it read-only, and
 * following something else is the switch dialog's uninstall + reinstall.
 */
describe('Hero, picking what an entry follows', () => {
	const STABLE = {
		name: 'stable',
		kind: 'ordered' as const,
		latest: 'v1.21.4',
		count: 2,
		members: ['v1.21.4', 'v1.21.0'],
	};
	const BETA = {
		name: 'beta',
		kind: 'ordered' as const,
		latest: 'v1.22.0-beta.2',
		count: 2,
		members: ['v1.22.0-beta.2', 'v1.22.0-beta.1'],
	};
	const NIGHTLY = { name: 'nightly', kind: 'pointer' as const, latest: 'nightly-latest' };
	const BARE = 'github.com/rabbyte/minecraft';

	/** A Discovered arrow, which core reports under its repository's default channel. */
	function discovered(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
		return detail({
			namespace: `${BARE}@stable`,
			selector: 'stable',
			selector_kind: 'channel',
			user_installed: false,
			state: 'absent',
			channels: [STABLE, BETA, NIGHTLY],
			...overrides,
		});
	}

	function renderWithNavigation(d: ArrowDetail) {
		const onIdentityChange = vi.fn();
		renderHero({ detail: d, onIdentityChange });
		return onIdentityChange;
	}

	describe('not yet in the library', () => {
		it('starts on the channel the identity follows, at its newest version', () => {
			renderHero({ detail: discovered() });
			expect(screen.getByRole('combobox', { name: 'Channel' })).toHaveTextContent('stable');
			expect(screen.getByRole('combobox', { name: 'Version' })).toHaveTextContent('v1.21.4');
		});

		it('lists every published channel, marking a pointer channel as rolling', async () => {
			const user = userEvent.setup();
			renderHero({ detail: discovered() });

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			expect(await screen.findByRole('option', { name: 'stable' })).toBeInTheDocument();
			expect(screen.getByRole('option', { name: 'beta' })).toBeInTheDocument();
			expect(screen.getByRole('option', { name: 'nightly (rolling)' })).toBeInTheDocument();
		});

		it('keeps a pick local -- nothing is sent until Add to Library', async () => {
			const user = userEvent.setup();
			renderHero({ detail: discovered() });

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));

			expect(apiFetch).not.toHaveBeenCalled();
			expect(screen.getByRole('combobox', { name: 'Version' })).toHaveTextContent('v1.22.0-beta.2');
		});

		it('registers the picked channel as the identity, with no body, and moves the page to it', async () => {
			const user = userEvent.setup();
			const onIdentityChange = renderWithNavigation(discovered());

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));
			await user.click(screen.getByRole('button', { name: 'Add to Library' }));

			await waitFor(() => expect(onIdentityChange).toHaveBeenCalledWith(`${BARE}@beta`));
			expect(apiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(`${BARE}@beta`)}`, {
				method: 'POST',
			});
		});

		it('registers a picked older version as a pin of exactly that ref', async () => {
			const user = userEvent.setup();
			renderWithNavigation(discovered());

			await user.click(screen.getByRole('combobox', { name: 'Version' }));
			await user.click(await screen.findByRole('option', { name: 'v1.21.0' }));
			await user.click(screen.getByRole('button', { name: 'Add to Library' }));

			await waitFor(() =>
				expect(apiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(`${BARE}@v1.21.0`)}`, {
					method: 'POST',
				})
			);
		});

		it('pins a ref typed into a pointer channel’s field, committed on Enter', async () => {
			const user = userEvent.setup();
			renderWithNavigation(discovered());

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'nightly (rolling)' }));
			const field = screen.getByRole('textbox', { name: 'Version' });
			expect(field).toHaveValue('nightly-latest');
			await user.clear(field);
			await user.type(field, 'a1b2c3d{Enter}');
			await user.click(screen.getByRole('button', { name: 'Add to Library' }));

			await waitFor(() =>
				expect(apiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(`${BARE}@a1b2c3d`)}`, {
					method: 'POST',
				})
			);
		});

		it('reverts a pointer channel’s field left blank instead of pinning nothing', async () => {
			const user = userEvent.setup();
			renderHero({ detail: discovered({ namespace: `${BARE}@nightly`, selector: 'nightly' }) });

			const field = screen.getByRole('textbox', { name: 'Version' });
			await user.clear(field);
			await user.tab();

			expect(screen.getByRole('textbox', { name: 'Version' })).toHaveValue('nightly-latest');
		});

		it('keeps the page where it is when the pick is the identity it already shows', async () => {
			const user = userEvent.setup();
			const onIdentityChange = renderWithNavigation(discovered());

			await user.click(screen.getByRole('button', { name: 'Add to Library' }));

			await waitFor(() =>
				expect(apiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(`${BARE}@stable`)}`, {
					method: 'POST',
				})
			);
			expect(onIdentityChange).not.toHaveBeenCalled();
		});

		it('renders no version options, without crashing, for an ordered channel with no members', async () => {
			const user = userEvent.setup();
			const noMembers = { name: 'edge', kind: 'ordered' as const, latest: 'edge-1' };
			renderHero({ detail: discovered({ namespace: `${BARE}@edge`, selector: 'edge', channels: [noMembers] }) });

			await user.click(screen.getByRole('combobox', { name: 'Version' }));
			expect(screen.queryAllByRole('option')).toHaveLength(0);
		});

		it('renders no picker, without crashing, when the arrow publishes no channels', () => {
			renderHero({ detail: discovered({ channels: [] }) });
			expect(screen.queryByRole('combobox', { name: 'Channel' })).not.toBeInTheDocument();
		});
	});

	describe('already in the library', () => {
		it('shows what the entry follows and what is installed, read-only', () => {
			renderHero({
				detail: detail({
					namespace: `${BARE}@stable`,
					selector: 'stable',
					selector_kind: 'channel',
					resolved_ref: 'v1.21.4',
					installed_at: '2026-09-01T00:00:00Z',
					channels: [STABLE, BETA],
				}),
			});

			expect(screen.queryByRole('combobox', { name: 'Channel' })).not.toBeInTheDocument();
			const summary = document.querySelector('[data-slot="selector-summary"]');
			expect(summary).toHaveTextContent('Channel');
			expect(summary).toHaveTextContent('stable');
			expect(summary).toHaveTextContent('Installed v1.21.4');
		});

		it('says what a not-yet-installed entry resolves to', () => {
			renderHero({
				detail: detail({
					namespace: `${BARE}@stable`,
					selector: 'stable',
					selector_kind: 'channel',
					resolved_ref: 'v1.21.4',
					state: 'absent',
					installed_at: undefined,
				}),
			});
			expect(document.querySelector('[data-slot="selector-summary"]')).toHaveTextContent('Resolves to v1.21.4');
		});

		it('never sends a PATCH to switch in place', async () => {
			const user = userEvent.setup();
			renderHero({
				detail: detail({ namespace: `${BARE}@stable`, selector: 'stable', channels: [STABLE, BETA] }),
			});

			await user.click(screen.getByRole('button', { name: 'Switch…' }));
			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));

			expect(mockApiFetch.mock.calls.some(([, init]) => (init as RequestInit)?.method === 'PATCH')).toBe(false);
		});

		it('switches only after confirming that it means reinstalling, then moves the page to the new identity', async () => {
			const user = userEvent.setup();
			const onIdentityChange = renderWithNavigation(
				detail({
					namespace: `${BARE}@stable`,
					selector: 'stable',
					selector_kind: 'channel',
					state: 'absent',
					channels: [STABLE, BETA],
				})
			);

			await user.click(screen.getByRole('button', { name: 'Switch…' }));
			const dialog = await screen.findByRole('dialog');
			expect(dialog).toHaveTextContent('Switching means reinstalling');
			expect(screen.getByRole('button', { name: 'Uninstall and reinstall' })).toBeDisabled();

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));
			expect(dialog).toHaveTextContent(`${BARE}@beta`);
			expect(apiFetch).not.toHaveBeenCalled();

			await user.click(screen.getByRole('button', { name: 'Uninstall and reinstall' }));

			await waitFor(() => expect(onIdentityChange).toHaveBeenCalledWith(`${BARE}@beta`));
			await waitFor(() =>
				expect(
					mockApiFetch.mock.calls.map(([path, init]) => `${(init as RequestInit).method} ${path}`)
				).toEqual([
					`POST /v0/arrow/${encodeURIComponent(`${BARE}@beta`)}`,
					`DELETE /v0/arrow/${encodeURIComponent(`${BARE}@stable`)}`,
				])
			);
			await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
		});

		it('keeps the dialog open and changes nothing when the new identity cannot be registered', async () => {
			mockApiFetch.mockRejectedValueOnce(new ApiError('fetch failed', 502));
			const user = userEvent.setup();
			const onIdentityChange = renderWithNavigation(
				detail({
					namespace: `${BARE}@stable`,
					selector: 'stable',
					state: 'absent',
					channels: [STABLE, NIGHTLY],
				})
			);

			await user.click(screen.getByRole('button', { name: 'Switch…' }));
			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'nightly (rolling)' }));
			const field = screen.getByRole('textbox', { name: 'Version' });
			await user.clear(field);
			await user.type(field, 'feat/typo{Enter}');
			await user.click(screen.getByRole('button', { name: 'Uninstall and reinstall' }));

			expect(await screen.findByRole('alert')).toHaveTextContent('nothing was changed: fetch failed');
			expect(screen.getByRole('dialog')).toHaveTextContent('Switching means reinstalling');
			expect(mockApiFetch.mock.calls.map(([path, init]) => `${(init as RequestInit).method} ${path}`)).toEqual([
				`POST /v0/arrow/${encodeURIComponent(`${BARE}@feat/typo`)}`,
			]);
			expect(onIdentityChange).not.toHaveBeenCalled();
		});

		it.each([
			['uninstall', 'still installed', true],
			['remove', "couldn't be removed from your library", true],
			['install', 'nothing is installed', false],
		] as const)(
			'says plainly what is left when the %s step fails, with the page already on the new identity',
			async (step, left, offersOld) => {
				useSelectorSwitchStore.setState({
					active: null,
					failure: { step, from: `${BARE}@stable`, to: `${BARE}@beta`, reason: 'boom' },
				});
				const user = userEvent.setup();
				const onIdentityChange = renderWithNavigation(detail({ namespace: `${BARE}@beta`, selector: 'beta' }));

				const dialog = await screen.findByRole('dialog');
				expect(dialog).toHaveTextContent("Couldn't switch");
				expect(dialog).toHaveTextContent(left);
				expect(dialog).toHaveTextContent('boom');

				if (offersOld) {
					await user.click(screen.getByRole('button', { name: 'Open the old version' }));
					expect(onIdentityChange).toHaveBeenCalledWith(`${BARE}@stable`);
				} else {
					expect(screen.queryByRole('button', { name: 'Open the old version' })).not.toBeInTheDocument();
					await user.keyboard('{Escape}');
				}
				await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
				expect(useSelectorSwitchStore.getState().failure).toBeNull();
			}
		);

		it('shows a late failure only on the pages of the identities it involves', () => {
			useSelectorSwitchStore.setState({
				active: null,
				failure: { step: 'install', from: `${BARE}@stable`, to: `${BARE}@beta`, reason: 'boom' },
			});
			renderHero({ detail: detail({ namespace: 'github.com/rabbyte/valheim@v1', selector: 'v1' }) });
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		});

		it('locks the new identity’s actions while the switch is still working on it', () => {
			useSelectorSwitchStore.setState({ active: { from: `${BARE}@stable`, to: `${BARE}@beta` }, failure: null });
			renderHero({ detail: detail({ namespace: `${BARE}@beta`, selector: 'beta', state: 'absent' }) });

			expect(screen.getByRole('button', { name: 'Install' })).toBeDisabled();
			expect(screen.getByRole('button', { name: 'Remove from Library' })).toBeDisabled();
		});

		it('leaves other pages’ actions alone during a switch', () => {
			useSelectorSwitchStore.setState({ active: { from: `${BARE}@stable`, to: `${BARE}@beta` }, failure: null });
			renderHero({
				detail: detail({ namespace: 'github.com/rabbyte/valheim@v1', selector: 'v1', state: 'absent' }),
			});

			expect(screen.getByRole('button', { name: 'Install' })).not.toBeDisabled();
		});

		it('dismisses a late failure without going anywhere', async () => {
			useSelectorSwitchStore.setState({
				active: null,
				failure: { step: 'remove', from: `${BARE}@stable`, to: `${BARE}@beta`, reason: 'boom' },
			});
			const user = userEvent.setup();
			const onIdentityChange = renderWithNavigation(detail({ namespace: `${BARE}@beta`, selector: 'beta' }));

			await user.click(await screen.findByRole('button', { name: 'Dismiss' }));

			await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
			expect(onIdentityChange).not.toHaveBeenCalled();
		});

		it('opens the switch dialog of a constraint row with nothing picked and Confirm disabled', async () => {
			const user = userEvent.setup();
			renderHero({
				detail: detail({
					namespace: `${BARE}@v1.*`,
					selector: 'v1.*',
					selector_kind: 'constraint',
					channels: [STABLE, BETA],
				}),
			});

			await user.click(screen.getByRole('button', { name: 'Switch…' }));

			expect(screen.getByRole('button', { name: 'Uninstall and reinstall' })).toBeDisabled();
			expect(screen.getByRole('dialog')).toHaveTextContent(`Pick what ${BARE}@v1.* should follow instead.`);
			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));
			expect(screen.getByRole('button', { name: 'Uninstall and reinstall' })).not.toBeDisabled();
		});

		it('shows a loading state, not a from == to note, while channels are still arriving', async () => {
			const user = userEvent.setup();
			renderHero({
				channelsLoading: true,
				detail: detail({ namespace: `${BARE}@stable`, selector: 'stable', channels: [STABLE] }),
			});

			await user.click(screen.getByRole('button', { name: 'Switch…' }));

			const dialog = screen.getByRole('dialog');
			expect(dialog).toHaveTextContent('Loading');
			expect(dialog).not.toHaveTextContent(`${BARE}@stable will be uninstalled`);
			expect(screen.getByRole('button', { name: 'Uninstall and reinstall' })).toBeDisabled();
		});

		it('cancels the switch without touching anything', async () => {
			const user = userEvent.setup();
			renderHero({
				detail: detail({ namespace: `${BARE}@stable`, selector: 'stable', channels: [STABLE, BETA] }),
			});

			await user.click(screen.getByRole('button', { name: 'Switch…' }));
			await user.click(screen.getByRole('button', { name: 'Cancel' }));

			await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
			expect(apiFetch).not.toHaveBeenCalled();
		});

		it('offers no switch while the entry is busy', () => {
			renderHero({
				detail: detail({
					namespace: `${BARE}@stable`,
					selector: 'stable',
					state: 'running',
					channels: [STABLE],
				}),
			});
			expect(screen.getByRole('button', { name: 'Switch…' })).toBeDisabled();
		});

		it('offers no switch for an arrow that publishes no channels', () => {
			renderHero({ detail: detail({ channels: [] }) });
			expect(screen.queryByRole('button', { name: 'Switch…' })).not.toBeInTheDocument();
		});
	});
});

describe('Hero, updating', () => {
	const AHEAD = { ref: 'v1.22.0', commit: 'abc1234' };

	it('lets quiver.core update like any other arrow, sending no variables', async () => {
		const user = userEvent.setup();
		renderHero({
			detail: detail({
				namespace: 'github.com/rabbytesoftware/quiver.core@stable',
				state: 'outdated',
				available: AHEAD,
				outdated: true,
			}),
		});

		expect(screen.getByText('Update available')).toBeInTheDocument();
		const update = screen.getByRole('button', { name: 'Update' });
		expect(update).toBeEnabled();
		await user.click(update);
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/update'), expect.anything())
		);
		expect(bodyOf(lastCall())).toEqual({ variables: {} });
	});

	it.each([200, 202])(
		're-reads the catalog after an update core answered %i, for the sidebar’s resolved ref',
		async (status) => {
			update.status = status;
			const refresh = vi.fn();
			useArrowStore.getState().setCatalogRefresh(refresh);
			const user = userEvent.setup();
			renderHero({ detail: detail({ state: 'outdated', available: AHEAD, outdated: true }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));

			await waitFor(() => expect(refresh).toHaveBeenCalled());
		}
	);

	it.each([
		['Add to Library', { user_installed: false, state: 'absent' as const }],
		['Uninstall', {}],
	])('re-reads the catalog after %s', async (button, overrides) => {
		const refresh = vi.fn();
		useArrowStore.getState().setCatalogRefresh(refresh);
		const user = userEvent.setup();
		renderHero({ detail: detail(overrides) });

		await user.click(screen.getByRole('button', { name: button }));

		await waitFor(() => expect(refresh).toHaveBeenCalled());
	});

	it('shows "Update available" and offers Update from ready as soon as something is available', () => {
		renderHero({ detail: detail({ state: 'ready', available: AHEAD, outdated: true }) });
		expect(screen.getByText('Update available')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Update' })).toBeInTheDocument();
	});

	it('offers no Update when nothing is available', () => {
		renderHero({ detail: detail({ state: 'ready', available: null }) });
		expect(screen.queryByRole('button', { name: 'Update' })).not.toBeInTheDocument();
	});

	it('says "Already up to date" when core answers 200, and waits for no run', async () => {
		update.status = 200;
		const user = userEvent.setup();
		const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
		const invalidateSpy = vi.spyOn(qc, 'invalidateQueries');
		render(
			<Hero
				detail={detail({ state: 'outdated', available: AHEAD, outdated: true })}
				onValueChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper(qc) }
		);

		await user.click(screen.getByRole('button', { name: 'Update' }));

		expect(await screen.findByRole('status')).toHaveTextContent('Already up to date');
		expect(screen.getByRole('button', { name: 'Update' })).not.toBeDisabled();
		expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['arrow'] });
	});

	it('clears "Already up to date" after a few seconds', async () => {
		update.status = 200;
		vi.useFakeTimers({ shouldAdvanceTime: true });
		try {
			const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
			renderHero({ detail: detail({ state: 'outdated', available: AHEAD, outdated: true }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));
			expect(await screen.findByRole('status')).toBeInTheDocument();

			await act(() => vi.advanceTimersByTimeAsync(5000));
			expect(screen.queryByRole('status')).not.toBeInTheDocument();
		} finally {
			vi.useRealTimers();
		}
	});

	it('treats a 202 as an update that started: no up-to-date note, the run reports itself', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'outdated', available: AHEAD, outdated: true }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/update'), expect.anything())
		);
		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it.each([422, 409])('offers to try again when core answers %i (a racing or unapplied update)', async (status) => {
		mockApiFetch.mockRejectedValueOnce(new ApiError('state violation', status));
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'outdated', available: AHEAD, outdated: true }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));

		const dialog = await screen.findByRole('dialog');
		expect(dialog).toHaveTextContent('Another update is underway');
		expect(dialog).toHaveTextContent('state violation');

		await user.click(screen.getByRole('button', { name: 'Try again' }));
		await waitFor(() => expect(mockApiFetch).toHaveBeenCalledTimes(2));
		expect(mockApiFetch.mock.calls[1][0]).toContain('/update');
	});

	it('closes the retry dialog without retrying', async () => {
		mockApiFetch.mockRejectedValueOnce(new ApiError('state violation', 422));
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'outdated', available: AHEAD, outdated: true }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));
		await screen.findByRole('dialog');
		await user.click(screen.getByRole('button', { name: 'Dismiss' }));

		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
		expect(mockApiFetch).toHaveBeenCalledTimes(1);
	});

	it('reports any other update failure once, with no retry offered', async () => {
		mockApiFetch.mockRejectedValueOnce(new ApiError('fetch failed', 502));
		const user = userEvent.setup();
		renderHero({ detail: detail({ state: 'outdated', available: AHEAD, outdated: true }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));

		expect(await screen.findByText('fetch failed')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Try again' })).not.toBeInTheDocument();
	});
});

describe('Hero, restart to apply', () => {
	const STAGED = { version: 'v1.22.0', staged_at: '2026-10-03T10:00:00Z' };
	const AHEAD = { ref: 'v1.22.0', commit: 'abc1234' };

	it.each([
		['quiver.core', 'github.com/rabbytesoftware/quiver.core@stable'],
		['an ordinary arrow', 'github.com/rabbyte/minecraft@v1.21.4'],
	])('offers Restart to apply instead of Update on %s once an update is staged', (_name, namespace) => {
		renderHero({ detail: detail({ namespace, available: AHEAD, outdated: true, pending_activation: STAGED }) });
		expect(screen.getByRole('button', { name: 'Restart to apply' })).toBeEnabled();
		expect(screen.queryByRole('button', { name: 'Update' })).not.toBeInTheDocument();
	});

	it('activates the staged update and shows a restarting state while the daemon comes back', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ pending_activation: STAGED }) });

		await user.click(screen.getByRole('button', { name: 'Restart to apply' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/activate'), expect.anything())
		);
		expect(await screen.findByRole('button', { name: /Restarting/ })).toBeDisabled();
	});

	it('goes back to Start once the new version reports nothing staged', async () => {
		const user = userEvent.setup();
		const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
		const props = { channelsLoading: false, onValueChange: vi.fn(), platform: PLATFORM, values: {} };
		const view = render(<Hero {...props} detail={detail({ pending_activation: STAGED })} />, {
			wrapper: wrapper(client),
		});
		await user.click(screen.getByRole('button', { name: 'Restart to apply' }));
		await screen.findByRole('button', { name: /Restarting/ });

		view.rerender(<Hero {...props} detail={detail({ resolved_ref: 'v1.22.0', pending_activation: null })} />);

		await waitFor(() => expect(screen.queryByRole('button', { name: /Restarting/ })).not.toBeInTheDocument());
		expect(screen.queryByRole('button', { name: 'Restart to apply' })).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled();
	});

	it('reports a refused activation instead of showing a restart', async () => {
		mockApiFetch.mockRejectedValueOnce(new ApiError('handover could not start', 500));
		const user = userEvent.setup();
		renderHero({ detail: detail({ pending_activation: STAGED }) });

		await user.click(screen.getByRole('button', { name: 'Restart to apply' }));

		expect(await screen.findByText('handover could not start')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Restarting/ })).not.toBeInTheDocument();
	});
});

/**
 * `isPlatformSupported` (an exact match, no fallback) is what both of these
 * are driven by -- `TARGET.platform` is `PLATFORM` by default, so every test
 * that overrides `platform` to something else exercises the "genuinely
 * unsupported" path, and every other test in this file (rendered with the
 * default matching platform) is itself proof the indicator/warning stay
 * silent on a supported arrow.
 */
describe('Hero, auto-made badge', () => {
	it('shows the badge beside the name for an inferred arrow, reachable by keyboard', () => {
		renderHero({ detail: detail({ origin: 'inferred', confidence: 'medium' }) });
		expect(screen.getByText('Auto-made')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Auto-made' })).toBeInTheDocument();
	});

	it.each([
		['declared', { origin: 'declared' as const }],
		['carrying no origin', {}],
	])('shows nothing for an arrow that is %s', (_name, extra) => {
		renderHero({ detail: detail(extra) });
		expect(screen.queryByText('Auto-made')).not.toBeInTheDocument();
	});
});

describe('Hero, platform support', () => {
	it('shows no platform-unsupported badge when the target matches the detected platform', () => {
		renderHero();
		expect(screen.queryByText(/Not available for/)).not.toBeInTheDocument();
	});

	it('shows a platform-unsupported badge naming the detected platform, when nothing matches', () => {
		renderHero({ platform: 'linux/amd64' });
		expect(screen.getByText('Not available for linux/amd64')).toBeInTheDocument();
	});

	it('adds a platform-supported arrow to the library immediately, with no warning', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ user_installed: false }) });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));

		await waitFor(() => expect(apiFetch).toHaveBeenCalled());
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('warns before adding an unsupported-platform arrow, instead of registering right away', async () => {
		const user = userEvent.setup();
		platformBackend('linux/amd64');
		renderHero({ detail: detail({ user_installed: false }), platform: 'linux/amd64' });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));

		expect(await screen.findByRole('dialog')).toHaveTextContent("isn't available for your platform");
		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('cancelling the platform warning leaves the arrow out of the library', async () => {
		const user = userEvent.setup();
		platformBackend('linux/amd64');
		renderHero({ detail: detail({ user_installed: false }), platform: 'linux/amd64' });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));
		await user.click(await screen.findByRole('button', { name: 'Cancel' }));

		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('"Add anyway" proceeds with the exact same registerArrow call an ordinary add makes', async () => {
		const user = userEvent.setup();
		platformBackend('linux/amd64');
		renderHero({ detail: detail({ user_installed: false }), platform: 'linux/amd64' });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));
		await user.click(await screen.findByRole('button', { name: 'Add anyway' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(
				expect.stringContaining(encodeURIComponent(detail().namespace)),
				expect.objectContaining({ method: 'POST' })
			)
		);
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	// Guards the regression a review caught: the add-to-library gate used to
	// read the `platform` prop directly, which can still be `useRealPlatform`'s
	// zero-latency UA guess if the click lands before its effect resolves --
	// silently skipping the warning for a genuinely unsupported arrow instead
	// of merely mis-rendering a badge. The `platform` prop here still says the
	// arrow is supported (matching `TARGET.platform`), but the backend --
	// standing in for `useRealPlatform`'s eventual, authoritative answer --
	// disagrees; the gate must trust the backend, not the prop.
	it('warns even when the platform prop still shows a match, if the real platform disagrees', async () => {
		const user = userEvent.setup();
		platformBackend('linux/amd64');
		renderHero({ detail: detail({ user_installed: false }) });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));

		expect(await screen.findByRole('dialog')).toHaveTextContent('linux/amd64');
		expect(apiFetch).not.toHaveBeenCalled();
	});
});

/**
 * Quiver updating itself, driven the way a person drives it: by clicking the
 * button on Quiver's own tile.
 *
 * `ARROW.md`'s update lifecycle fetches `${QUIVER_RELEASE_ASSET_URL}` and
 * verifies `${QUIVER_RELEASE_CHECKSUM}`, neither of which has a default, so
 * core rejects the execution outright unless the caller resolves both. This
 * suite is the proof that the real button does.
 */
describe('Hero, updating Quiver itself', () => {
	/** What core reports for the row when the click re-reads it: `available` or nothing ahead. */
	function coreReports(available: { ref: string; commit: string } | null) {
		mockApiFetch.mockImplementation((_path: string, init?: RequestInit) =>
			Promise.resolve(
				init?.method === undefined
					? ({
							namespace: selfDetail().namespace,
							resolved_ref: 'stable-1.0',
							...(available ? { available } : {}),
						} as never)
					: undefined
			)
		);
	}

	function updateCalls() {
		return mockApiFetch.mock.calls.filter(([path]) => String(path).endsWith('/update'));
	}

	beforeEach(() => coreReports({ ref: 'stable-1.1', commit: 'def' }));

	it('resolves the release asset and sends both variables core requires', async () => {
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'outdated' }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/update'), expect.anything())
		);
		expect(resolve).toHaveBeenCalledTimes(1);
		expect(bodyOf(lastCall())).toEqual({
			variables: {
				QUIVER_RELEASE_ASSET_URL: RESOLVED.url,
				QUIVER_RELEASE_CHECKSUM: RESOLVED.checksum,
			},
		});
	});

	// The ref the row sits at is the one being updated FROM, and the selector
	// (`stable`) names no release at all. The asset has to come from the
	// release the row moves to -- `available.ref` -- which is exactly why it
	// cannot be templated inside the manifest.
	it('sends the asset of the release being updated TO, never the installed ref or the selector', async () => {
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'outdated' }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));

		await waitFor(() => expect(updateCalls()).toHaveLength(1));
		expect(resolve).toHaveBeenCalledWith('stable-1.1');
		const body = bodyOf(lastCall()) as { variables: Record<string, string> };
		expect(body.variables.QUIVER_RELEASE_ASSET_URL).toContain('stable-1.1');
		expect(body.variables.QUIVER_RELEASE_ASSET_URL).not.toContain('stable-1.0');
	});

	// THE STALE-VALUE GUARD at the click site: a second update must resolve
	// again rather than ride on whatever the first one left behind, in this
	// component or inside core.
	it('re-resolves on a second click rather than reusing the first answer', async () => {
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'outdated' }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));
		await waitFor(() => expect(updateCalls()).toHaveLength(1));
		await user.click(screen.getByRole('button', { name: 'Update' }));
		await waitFor(() => expect(updateCalls()).toHaveLength(2));

		expect(resolve).toHaveBeenCalledTimes(2);
	});

	// The page's `available` came from a one-time read. The version check can
	// flip the runtime to outdated while the page is open, and only `state`
	// arrives live -- so the click decides from a fresh read, never from the
	// installed ref.
	it('decides from a fresh read when the page’s own detail knows of nothing ahead', async () => {
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'outdated', available: null, outdated: false }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));

		await waitFor(() => expect(updateCalls()).toHaveLength(1));
		expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(selfDetail().namespace)}`);
		expect(resolve).toHaveBeenCalledWith('stable-1.1');
		expect(resolve).not.toHaveBeenCalledWith('stable-1.0');
	});

	it('refuses to update, and refreshes the page, when the fresh read has nothing ahead', async () => {
		coreReports(null);
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
		const invalidateSpy = vi.spyOn(qc, 'invalidateQueries');
		render(
			<Hero detail={selfDetail({ state: 'outdated' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />,
			{ wrapper: wrapper(qc) }
		);

		await user.click(screen.getByRole('button', { name: 'Update' }));

		expect(await screen.findByText(/nothing newer to update to/)).toBeInTheDocument();
		expect(updateCalls()).toHaveLength(0);
		expect(resolve).not.toHaveBeenCalled();
		expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['arrow'] });
	});

	it('resolves for install too, from the ref the row resolved to', async () => {
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'absent', available: null, outdated: false }) });

		await user.click(screen.getByRole('button', { name: 'Install' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/install'), expect.anything())
		);
		expect(resolve).toHaveBeenCalledTimes(1);
		expect(resolve).toHaveBeenCalledWith('stable-1.0');
		expect(bodyOf(lastCall())).toEqual({
			variables: {
				QUIVER_RELEASE_ASSET_URL: RESOLVED.url,
				QUIVER_RELEASE_CHECKSUM: RESOLVED.checksum,
			},
		});
	});

	// uninstall's steps expand nothing but a defaulted path, so core requires
	// nothing -- and resolving would put a pointless network request in front
	// of removing an app the user may be removing BECAUSE they are offline.
	it('does not reach for the network to uninstall', async () => {
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'ready' }) });

		await user.click(screen.getByRole('button', { name: 'Uninstall' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/uninstall'), expect.anything())
		);
		expect(resolve).not.toHaveBeenCalled();
	});

	describe('when the asset cannot be resolved', () => {
		it('does not start an execution that core would refuse', async () => {
			const user = userEvent.setup();
			resolverAnswering({ reject: { kind: 'offline', detail: 'dns error' } });
			renderHero({ detail: selfDetail({ state: 'outdated' }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));

			await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
			expect(updateCalls()).toHaveLength(0);
		});

		it('says so in plain words, with the technical text underneath', async () => {
			const user = userEvent.setup();
			resolverAnswering({ reject: { kind: 'offline', detail: 'dns error: api.github.com' } });
			renderHero({ detail: selfDetail({ state: 'outdated' }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));

			expect(await screen.findByText(/Check your connection and try again/)).toBeInTheDocument();
			expect(screen.getByText(/dns error: api\.github\.com/)).toBeInTheDocument();
		});

		it('tells a rate-limited user to wait rather than blaming their connection', async () => {
			const user = userEvent.setup();
			resolverAnswering({ reject: { kind: 'rate_limited', detail: 'GitHub answered 403' } });
			renderHero({ detail: selfDetail({ state: 'outdated' }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));

			expect(await screen.findByText(/Try again in an hour/)).toBeInTheDocument();
		});

		it('names the real problem when this release has no bundle for this machine', async () => {
			const user = userEvent.setup();
			resolverAnswering({ reject: { kind: 'no_asset', detail: 'no *.AppImage asset' } });
			renderHero({ detail: selfDetail({ state: 'outdated' }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));

			expect(await screen.findByText(/doesn't include a download for this computer/)).toBeInTheDocument();
		});

		// The policy, at the surface a person actually sees: an unverifiable
		// asset stops the update rather than being installed with a warning.
		it('refuses an unverifiable download and explains why', async () => {
			const user = userEvent.setup();
			resolverAnswering({ ...RESOLVED, checksum: null });
			renderHero({ detail: selfDetail({ state: 'outdated' }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));

			expect(await screen.findByText(/can't be verified/)).toBeInTheDocument();
			expect(updateCalls()).toHaveLength(0);
		});

		// A failed resolution must not leave the button spinning forever: the
		// user has to be able to try again once they are back online.
		it('lets the user try again', async () => {
			const user = userEvent.setup();
			const resolve = vi
				.fn()
				.mockRejectedValueOnce({ kind: 'offline', detail: 'dns error' })
				.mockResolvedValueOnce(RESOLVED);
			installBackend({ resolveReleaseAsset: resolve } as unknown as Backend);
			renderHero({ detail: selfDetail({ state: 'outdated' }) });

			await user.click(screen.getByRole('button', { name: 'Update' }));
			expect(await screen.findByRole('dialog')).toBeInTheDocument();

			await user.keyboard('{Escape}');
			await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());

			await user.click(screen.getByRole('button', { name: 'Update' }));
			await waitFor(() =>
				expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/update'), expect.anything())
			);
		});
	});

	describe('inference notes', () => {
		it('shows no notes for a hand-written arrow', () => {
			renderHero({ detail: detail({ origin: 'declared' }) });
			expect(screen.queryByText('Things to check')).not.toBeInTheDocument();
			expect(screen.queryByText(/Confidence/)).not.toBeInTheDocument();
		});

		it('shows nothing extra when the core sends no origin at all', () => {
			renderHero();
			expect(screen.queryByText(/Confidence/)).not.toBeInTheDocument();
		});

		it('shows the confidence and the warnings for an inferred arrow', () => {
			renderHero({
				detail: detail({ origin: 'inferred', confidence: 'medium', warnings: ['unpinned_rolling_tag'] }),
			});
			expect(screen.getByText('Confidence: medium')).toBeInTheDocument();
			expect(screen.getByText('Things to check')).toBeInTheDocument();
			expect(screen.getByText(/rolling tag/)).toBeInTheDocument();
		});
	});
});
