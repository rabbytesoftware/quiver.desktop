import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';
import type { ArrowDetail, ArrowLifecycle, ArrowTarget } from '@/domain/arrow';
import type { ResolvedReleaseAsset } from '@/domain/release';
import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { apiFetch } from '@/lib/transport/api';
import type { Backend } from '@/lib/transport/backend';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { Hero } from './hero';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));
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
		installed_ref: 'v1.21.4',
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
	render(<Hero detail={detail()} onValueChange={onValueChange} platform={PLATFORM} values={{}} {...props} />, {
		wrapper: wrapper(),
	});
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

/** Quiver's own row, at the ref an update would be started FROM. */
function selfDetail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return detail({
		namespace: `${QUIVER_DESKTOP_NAMESPACE}@stable-1.0`,
		name: 'Quiver',
		variables: [
			{ name: 'QUIVER_RELEASE_ASSET_URL', description: 'Download URL.', type: 'string', default: '' },
			{ name: 'QUIVER_RELEASE_CHECKSUM', description: 'SHA-256.', type: 'string', default: '' },
		],
		installed_ref: 'stable-1.0',
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
	installBackend({ resolveReleaseAsset } as unknown as Backend);
	return resolveReleaseAsset;
}

beforeEach(() => {
	mockApiFetch.mockReset();
	mockApiFetch.mockResolvedValue(undefined);
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
			<Hero detail={detail({ user_installed: false })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />,
			{ wrapper: wrapper(qc) }
		);

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));

		await waitFor(() => expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['arrow'] }));
	});

	it('invalidates the arrow-detail query cache after Remove from Library succeeds too', async () => {
		const user = userEvent.setup();
		const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
		const invalidateSpy = vi.spyOn(qc, 'invalidateQueries');
		render(<Hero detail={detail({ state: 'absent' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />, {
			wrapper: wrapper(qc),
		});

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

	it('renders the license, with no channel or version switcher when there are no channels to show', () => {
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

	it('registers with no JSON body at all when the arrow has no channels to choose from', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ user_installed: false, channels: [] }) });

		await user.click(screen.getByRole('button', { name: 'Add to Library' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalled());
		expect(lastCall()[1]).toEqual({ method: 'POST' });
	});

	it('sequences Restart as stop then, once the live state reaches ready, execute -- not immediately after stop resolves', async () => {
		const user = userEvent.setup();
		const running = detail({ state: 'running' });
		const { rerender } = render(<Hero detail={running} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />, {
			wrapper: wrapper(),
		});

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/stop'), expect.anything()));

		// Only `stop` has fired -- `execute` must not fire until the parent
		// re-renders Hero with the live state actually at `ready`.
		expect(apiFetch).not.toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.anything());

		rerender(<Hero detail={detail({ state: 'ready' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />);

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.anything())
		);
	});

	it('does not fire the restart follow-up when the arrow reaches ready without a restart in flight', async () => {
		const { rerender } = render(
			<Hero detail={detail({ state: 'running' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />,
			{ wrapper: wrapper() }
		);
		rerender(<Hero detail={detail({ state: 'ready' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />);
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

	it('clears the restart-in-flight flag (not just pendingKind) when the stop leg of a restart itself rejects', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('offline'));
		const user = userEvent.setup();
		const { rerender } = render(
			<Hero detail={detail({ state: 'running' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />,
			{ wrapper: wrapper() }
		);

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		await waitFor(() => expect(screen.getByRole('button', { name: 'Restart' })).not.toBeDisabled());

		// If the failed restart's flag were left set, this transition to ready
		// would wrongly fire `execute` on its own.
		mockApiFetch.mockClear();
		rerender(<Hero detail={detail({ state: 'ready' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />);
		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('clears pendingKind even when restart’s second leg (execute, once ready) itself rejects', async () => {
		const user = userEvent.setup();
		const { rerender } = render(
			<Hero detail={detail({ state: 'running' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />,
			{ wrapper: wrapper() }
		);

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/stop'), expect.anything()));

		// The arrow reaching `ready` is what triggers the execute leg -- once
		// there, `computeActions` for `ready` no longer even offers a "Restart"
		// button (only `running`/`stopping`/`draining` do), so the real
		// assertion is that pendingKind was still cleared despite the
		// rejection: the newly-shown "Start" action must not be stuck disabled.
		mockApiFetch.mockRejectedValueOnce(new Error('offline'));
		rerender(<Hero detail={detail({ state: 'ready' })} onValueChange={vi.fn()} platform={PLATFORM} values={{}} />);

		await waitFor(() => expect(screen.getByRole('button', { name: 'Start' })).not.toBeDisabled());
	});
});

/**
 * The Channel/Version pair replaces the old "installed versions" switcher
 * entirely (see CLAUDE.md-adjacent design notes for the sibling-version
 * navigation this used to do): it answers "what channel is THIS install
 * tracking, and what's available inside it", scoped to `detail.channels`
 * only -- never the reactive store's other installed copies of the arrow.
 */
describe('Hero, the Channel and Version switchers', () => {
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

	it('renders channel options from detail.channels, defaulting to the currently tracked one', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ channel: 'stable', channels: [STABLE, BETA] }) });

		expect(screen.getByRole('combobox', { name: 'Channel' })).toHaveTextContent('stable');
		await user.click(screen.getByRole('combobox', { name: 'Channel' }));
		expect(await screen.findByRole('option', { name: 'stable' })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: 'beta' })).toBeInTheDocument();
	});

	it('defaults to the first published channel when the arrow tracks none yet', () => {
		renderHero({ detail: detail({ channel: undefined, channels: [BETA, STABLE], user_installed: false }) });
		expect(screen.getByRole('combobox', { name: 'Channel' })).toHaveTextContent('beta');
	});

	it('renders the ordered channel’s members as version options, highest precedence pre-selected', () => {
		renderHero({ detail: detail({ channel: 'stable', channels: [STABLE] }) });
		expect(screen.getByRole('combobox', { name: 'Version' })).toHaveTextContent('v1.21.4');
	});

	it('renders no version options, without crashing, for an ordered channel that carries no members', async () => {
		const user = userEvent.setup();
		const noMembers = { name: 'edge', kind: 'ordered' as const, latest: 'edge-1' };
		renderHero({ detail: detail({ channel: 'edge', channels: [noMembers] }) });

		const versionSelect = screen.getByRole('combobox', { name: 'Version' });
		await user.click(versionSelect);
		expect(screen.queryAllByRole('option')).toHaveLength(0);
	});

	it('marks a pointer channel in the picker and renders an editable version field, pre-filled with latest', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ channel: 'nightly', channels: [STABLE, NIGHTLY] }) });

		await user.click(screen.getByRole('combobox', { name: 'Channel' }));
		expect(await screen.findByRole('option', { name: 'nightly (rolling)' })).toBeInTheDocument();

		// A pointer channel (a branch, a rolling tag) is open-ended -- any ref
		// under it should be pinnable, not just whatever `latest` resolves to
		// right now -- so this is a free-text field, not a closed dropdown.
		const versionField = screen.getByRole('textbox', { name: 'Version' });
		expect(versionField).toHaveValue('nightly-latest');
		expect(versionField).not.toBeDisabled();
	});

	it('selecting a different channel swaps in that channel’s own version options', async () => {
		const user = userEvent.setup();
		renderHero({ detail: detail({ channel: 'stable', channels: [STABLE, BETA] }) });

		await user.click(screen.getByRole('combobox', { name: 'Channel' }));
		await user.click(await screen.findByRole('option', { name: 'beta' }));

		expect(screen.getByRole('combobox', { name: 'Version' })).toHaveTextContent('v1.22.0-beta.2');
		await user.click(screen.getByRole('combobox', { name: 'Version' }));
		expect(await screen.findByRole('option', { name: 'v1.22.0-beta.2' })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: 'v1.22.0-beta.1' })).toBeInTheDocument();
	});

	it('renders neither switcher, without crashing, when the arrow has no channels at all', () => {
		renderHero({ detail: detail({ channel: undefined, channels: [] }) });
		expect(screen.queryByRole('combobox', { name: 'Channel' })).not.toBeInTheDocument();
		expect(screen.queryByRole('combobox', { name: 'Version' })).not.toBeInTheDocument();
	});

	describe('not yet installed', () => {
		it('keeps a channel pick purely local -- no network call is made', async () => {
			const user = userEvent.setup();
			renderHero({
				detail: detail({ user_installed: false, state: 'absent', channel: 'stable', channels: [STABLE, BETA] }),
			});

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));

			expect(apiFetch).not.toHaveBeenCalled();
			expect(screen.getByRole('combobox', { name: 'Channel' })).toHaveTextContent('beta');
		});

		it('keeps a version pick purely local too -- only the channel is threaded into Add to Library, never a pinned ref', async () => {
			const user = userEvent.setup();
			renderHero({
				detail: detail({ user_installed: false, state: 'absent', channel: 'stable', channels: [STABLE] }),
			});

			await user.click(screen.getByRole('combobox', { name: 'Version' }));
			await user.click(await screen.findByRole('option', { name: 'v1.21.0' }));

			expect(apiFetch).not.toHaveBeenCalled();
			expect(screen.getByRole('combobox', { name: 'Version' })).toHaveTextContent('v1.21.0');
		});

		it('threads the picked channel into the Add to Library call', async () => {
			const user = userEvent.setup();
			renderHero({
				detail: detail({ user_installed: false, state: 'absent', channel: 'stable', channels: [STABLE, BETA] }),
			});

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));

			await user.click(screen.getByRole('button', { name: 'Add to Library' }));
			await waitFor(() => expect(apiFetch).toHaveBeenCalled());
			expect(bodyOf(lastCall())).toEqual({ channel: 'beta' });
		});
	});

	describe('already installed', () => {
		it('calls the switch-channel mutation and invalidates the arrow-detail query when a different channel is picked', async () => {
			const user = userEvent.setup();
			const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
			const invalidateSpy = vi.spyOn(qc, 'invalidateQueries');
			render(
				<Hero
					detail={detail({ user_installed: true, channel: 'stable', channels: [STABLE, BETA] })}
					onValueChange={vi.fn()}
					platform={PLATFORM}
					values={{}}
				/>,
				{ wrapper: wrapper(qc) }
			);

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));

			await waitFor(() =>
				expect(apiFetch).toHaveBeenCalledWith(
					expect.stringContaining(encodeURIComponent(detail().namespace)),
					expect.objectContaining({ method: 'PATCH' })
				)
			);
			expect(bodyOf(lastCall())).toEqual({ channel: 'beta', ref: 'v1.22.0-beta.2' });
			await waitFor(() => expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['arrow'] }));
		});

		it('calls the switch-channel mutation when only the version changes, keeping the same channel', async () => {
			const user = userEvent.setup();
			renderHero({ detail: detail({ user_installed: true, channel: 'stable', channels: [STABLE] }) });

			await user.click(screen.getByRole('combobox', { name: 'Version' }));
			await user.click(await screen.findByRole('option', { name: 'v1.21.0' }));

			await waitFor(() =>
				expect(apiFetch).toHaveBeenCalledWith(
					expect.stringContaining(encodeURIComponent(detail().namespace)),
					expect.objectContaining({ method: 'PATCH' })
				)
			);
			expect(bodyOf(lastCall())).toEqual({ channel: 'stable', ref: 'v1.21.0' });
		});

		it('sends the pointer channel’s own latest as ref, since it is the only version there is to pin', async () => {
			const user = userEvent.setup();
			renderHero({ detail: detail({ user_installed: true, channel: 'stable', channels: [STABLE, NIGHTLY] }) });

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'nightly (rolling)' }));

			await waitFor(() => expect(apiFetch).toHaveBeenCalled());
			expect(bodyOf(lastCall())).toEqual({ channel: 'nightly', ref: 'nightly-latest' });
		});

		it('commits a typed pointer-channel ref on blur', async () => {
			const user = userEvent.setup();
			renderHero({ detail: detail({ user_installed: true, channel: 'nightly', channels: [STABLE, NIGHTLY] }) });

			const versionField = screen.getByRole('textbox', { name: 'Version' });
			await user.clear(versionField);
			await user.type(versionField, 'a1b2c3d');
			await user.tab();

			await waitFor(() => expect(apiFetch).toHaveBeenCalled());
			expect(bodyOf(lastCall())).toEqual({ channel: 'nightly', ref: 'a1b2c3d' });
		});

		it('commits a typed pointer-channel ref on Enter, not on every keystroke', async () => {
			const user = userEvent.setup();
			renderHero({ detail: detail({ user_installed: true, channel: 'nightly', channels: [STABLE, NIGHTLY] }) });

			const versionField = screen.getByRole('textbox', { name: 'Version' });
			await user.clear(versionField);
			await user.type(versionField, 'feature-branch');
			expect(apiFetch).not.toHaveBeenCalled();

			await user.keyboard('{Enter}');
			await waitFor(() => expect(apiFetch).toHaveBeenCalled());
			expect(bodyOf(lastCall())).toEqual({ channel: 'nightly', ref: 'feature-branch' });
		});

		it('reverts to the last real value instead of committing a blank pointer-channel field', async () => {
			const user = userEvent.setup();
			renderHero({ detail: detail({ user_installed: true, channel: 'nightly', channels: [STABLE, NIGHTLY] }) });

			const versionField = screen.getByRole('textbox', { name: 'Version' });
			await user.clear(versionField);
			await user.tab();

			expect(apiFetch).not.toHaveBeenCalled();
			expect(screen.getByRole('textbox', { name: 'Version' })).toHaveValue('nightly-latest');
		});

		// The exact synchronous-double-click regression (both handlers firing
		// before `isPending` ever reaches a render) is covered at the hook
		// level in `use-channel-selection.test.ts` -- `user.click` here always
		// flushes React's render in between, so by the time a second `click`
		// could fire, the first one's `isPending` has already disabled the
		// other select for real (see "disables both selects while a channel
		// switch is in flight" below).

		it('does not crash and leaves the selects usable again when the channel switch is rejected', async () => {
			mockApiFetch.mockRejectedValueOnce(new Error('offline'));
			const user = userEvent.setup();
			renderHero({ detail: detail({ user_installed: true, channel: 'stable', channels: [STABLE, BETA] }) });

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));

			await waitFor(() => expect(apiFetch).toHaveBeenCalled());
			await waitFor(() => expect(screen.getByRole('combobox', { name: 'Channel' })).not.toBeDisabled());
		});

		it('disables both selects while a channel switch is in flight', async () => {
			let resolveSwitch: (value: undefined) => void = () => {};
			mockApiFetch.mockImplementationOnce(
				() =>
					new Promise((resolve) => {
						resolveSwitch = resolve;
					})
			);
			const user = userEvent.setup();
			renderHero({ detail: detail({ user_installed: true, channel: 'stable', channels: [STABLE, BETA] }) });

			await user.click(screen.getByRole('combobox', { name: 'Channel' }));
			await user.click(await screen.findByRole('option', { name: 'beta' }));

			await waitFor(() => expect(screen.getByRole('combobox', { name: 'Channel' })).toBeDisabled());
			expect(screen.getByRole('combobox', { name: 'Version' })).toBeDisabled();

			resolveSwitch(undefined);
			await waitFor(() => expect(screen.getByRole('combobox', { name: 'Channel' })).not.toBeDisabled());
		});
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

	// The ref the row sits at is the one being updated FROM. The asset has to
	// come from the newest release, not from this ref -- which is exactly why
	// it cannot be templated inside the manifest.
	it('sends the asset of the release being updated TO, not of the installed ref', async () => {
		const user = userEvent.setup();
		resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'outdated' }) });

		await user.click(screen.getByRole('button', { name: 'Update' }));

		await waitFor(() => expect(apiFetch).toHaveBeenCalled());
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
		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(1));
		await user.click(screen.getByRole('button', { name: 'Update' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(2));

		expect(resolve).toHaveBeenCalledTimes(2);
	});

	it('resolves for install too, which is the same fetch step', async () => {
		const user = userEvent.setup();
		const resolve = resolverAnswering(RESOLVED);
		renderHero({ detail: selfDetail({ state: 'absent' }) });

		await user.click(screen.getByRole('button', { name: 'Install' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/install'), expect.anything())
		);
		expect(resolve).toHaveBeenCalledTimes(1);
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
			expect(apiFetch).not.toHaveBeenCalled();
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
			expect(apiFetch).not.toHaveBeenCalled();
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
});
