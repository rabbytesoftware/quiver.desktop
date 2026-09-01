import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';
import type { ArrowDetail, ArrowLifecycle, ArrowTarget } from '@/domain/arrow';
import { apiFetch } from '@/lib/transport/api';

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
		versions: [{ ref: 'v1.21.4', version: '1.21.4', state: 'ready' }],
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
	const onVersionChange = vi.fn();
	render(
		<Hero
			detail={detail()}
			onValueChange={onValueChange}
			onVersionChange={onVersionChange}
			platform={PLATFORM}
			values={{}}
			{...props}
		/>,
		{ wrapper: wrapper() }
	);
	return { onValueChange, onVersionChange };
}

beforeEach(() => {
	mockApiFetch.mockReset();
	mockApiFetch.mockResolvedValue(undefined);
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
				detail={detail({ user_installed: false })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
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
				detail={detail({ state: 'absent' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper(qc) }
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

	it('renders the license, and the version switcher when versions are present', () => {
		renderHero();
		expect(screen.getByText('MIT')).toBeInTheDocument();
		expect(screen.getByRole('combobox', { name: 'Version' })).toBeInTheDocument();
	});

	it('omits the version switcher entirely when there are no versions to switch between', () => {
		renderHero({ detail: detail({ versions: [] }) });
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

	it('sequences Restart as stop then, once the live state reaches ready, execute -- not immediately after stop resolves', async () => {
		const user = userEvent.setup();
		const running = detail({ state: 'running' });
		const { rerender } = render(
			<Hero detail={running} onValueChange={vi.fn()} onVersionChange={vi.fn()} platform={PLATFORM} values={{}} />,
			{ wrapper: wrapper() }
		);

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith(expect.stringContaining('/stop'), expect.anything()));

		// Only `stop` has fired -- `execute` must not fire until the parent
		// re-renders Hero with the live state actually at `ready`.
		expect(apiFetch).not.toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.anything());

		rerender(
			<Hero
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
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
				detail={detail({ state: 'running' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper() }
		);
		rerender(
			<Hero
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
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
			<Hero
				detail={detail({ state: 'running' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>,
			{ wrapper: wrapper() }
		);

		await user.click(screen.getByRole('button', { name: 'Restart' }));
		await waitFor(() => expect(screen.getByRole('button', { name: 'Restart' })).not.toBeDisabled());

		// If the failed restart's flag were left set, this transition to ready
		// would wrongly fire `execute` on its own.
		mockApiFetch.mockClear();
		rerender(
			<Hero
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>
		);
		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('clears pendingKind even when restart’s second leg (execute, once ready) itself rejects', async () => {
		const user = userEvent.setup();
		const { rerender } = render(
			<Hero
				detail={detail({ state: 'running' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
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
		mockApiFetch.mockRejectedValueOnce(new Error('offline'));
		rerender(
			<Hero
				detail={detail({ state: 'ready' })}
				onValueChange={vi.fn()}
				onVersionChange={vi.fn()}
				platform={PLATFORM}
				values={{}}
			/>
		);

		await waitFor(() => expect(screen.getByRole('button', { name: 'Start' })).not.toBeDisabled());
	});

	it('calls onVersionChange when a different version is picked', async () => {
		const user = userEvent.setup();
		const { onVersionChange } = renderHero({
			detail: detail({
				versions: [
					{ ref: 'v1.21.4', version: '1.21.4', state: 'ready' },
					{ ref: 'v1.20.1', version: '1.20.1', state: 'ready' },
				],
			}),
		});

		await user.click(screen.getByRole('combobox', { name: 'Version' }));
		await user.click(await screen.findByRole('option', { name: 'v1.20.1' }));
		expect(onVersionChange).toHaveBeenCalledWith('v1.20.1');
	});
});
