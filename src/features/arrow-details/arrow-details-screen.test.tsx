import { useState } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';
import { installMockResizeObserver, MockResizeObserver } from '@/__mocks__/mock-resize-observer';
import { useArrowStore } from '@/lib/core-store';
import type { ArrowDetailDTO, ArrowManifestDTO } from '@/lib/core-store/dtos/v0/arrow';
import { apiFetch, ApiError } from '@/lib/transport/api';

import { ArrowDetailsScreen } from './arrow-details-screen';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn() };
});
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

const NS = 'github.com/rabbyte/minecraft@v1.21.4';

const DETAIL: ArrowDetailDTO = {
	namespace: 'github.com/rabbyte/minecraft',
	name: 'Minecraft Server',
	version: '1.21.4',
	description: 'A vanilla Minecraft Java Edition server.',
	license: 'MIT',
	state: 'ready',
	tags: ['game'],
	installed_ref: 'v1.21.4',
	installed_at: '2026-05-09T21:26:59Z',
	user_installed: true,
	active_run: null,
	last_return: null,
};

const MANIFEST: ArrowManifestDTO = {
	namespace: 'github.com/rabbyte/minecraft',
	name: 'Minecraft Server',
	description: 'A vanilla Minecraft Java Edition server.',
	tags: ['game'],
	variables: [{ name: 'server-name', description: 'Shown in the list.', type: 'string', default: 'My Server' }],
	// The matching target ('linux/amd64' -- jsdom's default `navigator.userAgent`
	// doesn't match the macOS pattern `currentPlatform()` looks for, so it
	// resolves to `linux/amd64`, its documented fallback) is listed SECOND,
	// after a decoy 'darwin/arm64' entry with different data. That ordering is
	// deliberate: it's what makes the "picks the target matching platform, not
	// just the first one" test below actually prove platform-matching drove
	// the choice, rather than passing by coincidence of object key order.
	targets: {
		'darwin/arm64': {
			requirements: { cpu_cores: 4, memory_gb: 8, disk_gb: 20 },
			lifecycle: {
				install: [],
				update: [],
				execute: [runStep('Start process (darwin)')],
				stop: [],
				uninstall: [],
			},
			methods: {},
		},
		'linux/amd64': {
			requirements: { cpu_cores: 2, memory_gb: 4, disk_gb: 10 },
			lifecycle: {
				install: [runStep('Fetch archive')],
				update: [runStep('Fetch new version')],
				execute: [runStep('Start process')],
				stop: [signalStep('Signal process')],
				uninstall: [runStep('Remove workdir')],
			},
			methods: {
				backup: {
					name: 'backup',
					description: 'Snapshot the world.',
					available_in: ['ready', 'running'],
					steps: [],
				},
			},
		},
	},
	manifest: {
		url: 'https://github.com/rabbyte/minecraft',
		maintainers: [{ name: 'rabbyte' }],
		credits: [],
		media: {},
		netbridge: [{ name: 'game', protocol: 'tcp', default: 25565, required: true }],
	},
};

function mockDetailAndManifest(
	detail: ArrowDetailDTO = DETAIL,
	manifest: ArrowManifestDTO = MANIFEST,
	readme: string | null = null,
	dependencies: { namespace: string; type: 'tool' | 'service' }[] = [],
	dependents: string[] = []
) {
	mockApiFetch.mockImplementation((path: string) => {
		if (path.endsWith('/readme')) {
			return readme === null
				? Promise.reject(new ApiError('not found', 404))
				: Promise.resolve({ namespace: path.replace('/v0/arrow/', '').replace('/readme', ''), readme });
		}
		if (path.endsWith('/manifest')) return Promise.resolve(manifest);
		if (path.endsWith('/dependencies')) return Promise.resolve({ namespace: NS, dependencies });
		if (path.endsWith('/dependents')) return Promise.resolve({ namespace: NS, dependents });
		return Promise.resolve(detail);
	});
}

function renderScreen(namespace: string) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
	const rootRoute = createRootRoute({
		component: () => <ArrowDetailsScreen namespace={namespace} />,
	});
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});
	const router = createRouter({
		routeTree: rootRoute.addChildren([arrowRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});

	return {
		router,
		...render(
			<QueryClientProvider client={client}>
				<RouterProvider router={router} />
			</QueryClientProvider>
		),
	};
}

beforeEach(() => {
	mockApiFetch.mockReset();
	useArrowStore.getState().reset();
});

describe('ArrowDetailsScreen', () => {
	it('shows a loading state before the fetch resolves', async () => {
		mockApiFetch.mockReturnValue(new Promise(() => {}));
		renderScreen(NS);
		expect(await screen.findByText('Loading…')).toBeInTheDocument();
	});

	it('shows an error state when the fetch fails', async () => {
		mockApiFetch.mockRejectedValue(new Error('offline'));
		renderScreen(NS);
		expect(await screen.findByText("Couldn't load this arrow.")).toBeInTheDocument();
	});

	it('renders the hero and defaults to the Overview tab (Details) for a healthy ready arrow', async () => {
		mockDetailAndManifest();
		renderScreen(NS);

		expect(await screen.findByRole('heading', { name: 'Minecraft Server' })).toBeInTheDocument();
		expect(screen.getByText('Requirements')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Start' })).toBeInTheDocument();
	});

	it('renders the readme instead of Details on Overview, and moves Details into the rail, when GET /v0/arrow/:ns/readme has one', async () => {
		mockDetailAndManifest(DETAIL, MANIFEST, '## About\n\nA vanilla Java Edition server.');
		renderScreen(NS);

		expect(await screen.findByRole('heading', { name: 'About' })).toBeInTheDocument();
		expect(screen.getByText('A vanilla Java Edition server.')).toBeInTheDocument();
		expect(screen.getByText('Requirements')).toBeInTheDocument();
	});

	it('keeps Details in the rail visible after switching away from Overview, when there is a README', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest(DETAIL, MANIFEST, '## About');
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		expect(screen.getByText('Requirements')).toBeInTheDocument();

		await user.click(screen.getByRole('tab', { name: 'Methods' }));
		expect(await screen.findByText('backup')).toBeInTheDocument();
		expect(screen.getByText('Requirements')).toBeInTheDocument();
	});

	it('shows the full Settings panel in the rail when the arrow declares variables, not just a summary', async () => {
		mockDetailAndManifest(DETAIL, MANIFEST, '## About');
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		expect(screen.getByText('1 setting')).toBeInTheDocument();
		expect(screen.getByRole('textbox', { name: 'server-name' })).toBeInTheDocument();
	});

	it('shows no rail at all when the arrow has no README and no settings', async () => {
		mockDetailAndManifest(DETAIL, { ...MANIFEST, variables: [] });
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		// Details renders once, as Overview's own content, not duplicated into a rail.
		expect(screen.getAllByText('Requirements')).toHaveLength(1);
		expect(screen.queryByText('1 setting')).not.toBeInTheDocument();
		expect(screen.getByTestId('arrow-detail-layout')).not.toHaveClass('grid');
		expect(screen.getByTestId('arrow-detail-layout')).not.toHaveClass('flex-col');
	});

	it('requests the readme from the bare namespace, not namespace@ref', async () => {
		mockDetailAndManifest(DETAIL, MANIFEST, '## About');
		renderScreen(NS);

		await waitFor(() =>
			expect(mockApiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Frabbyte%2Fminecraft/readme')
		);
	});

	it('picks the target matching the current platform for Requirements/Methods, not just the first one', async () => {
		mockDetailAndManifest();
		renderScreen(NS);

		// darwin/arm64 declares cpu_cores: 2 -- linux/amd64 (listed second in
		// the fixture) declares 4. This only passes if platform selection,
		// not target array order, drove which one rendered.
		expect(await screen.findByText('2 cores')).toBeInTheDocument();
	});

	it('switches tabs on click, and Methods lists the manifest-declared custom methods', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest();
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await user.click(screen.getByRole('tab', { name: 'Methods' }));
		expect(await screen.findByText('backup')).toBeInTheDocument();
		expect(screen.getByText('Snapshot the world.')).toBeInTheDocument();
	});

	it('seeds variable values from the manifest defaults, used when Start is invoked', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest();
		renderScreen(NS);

		await user.click(await screen.findByRole('button', { name: 'Start' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(
				expect.stringContaining('/execute'),
				expect.objectContaining({ body: JSON.stringify({ variables: { 'server-name': 'My Server' } }) })
			)
		);
	});

	it('defaults to the Activity tab when a run is already in flight on load', async () => {
		mockDetailAndManifest({
			...DETAIL,
			active_run: {
				method: 'execute',
				variables: {},
				steps: [{ index: 0, title: 'Start process', status: 'running', type: 'run' }],
			},
		});
		renderScreen(NS);

		expect(await screen.findByText('Start process')).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Activity', selected: true })).toBeInTheDocument();
	});

	it('defaults to the Activity tab when the last run failed', async () => {
		mockDetailAndManifest({
			...DETAIL,
			last_return: { method: 'install', outcome: 'failed', variables: {}, steps: [] },
		});
		renderScreen(NS);

		expect(await screen.findByRole('tab', { name: 'Activity', selected: true })).toBeInTheDocument();
	});

	it('resets to Overview when the namespace changes to a different arrow, even if you were on Methods', async () => {
		const user = userEvent.setup();
		const OTHER_NS = 'github.com/rabbyte/other@v1.0.0';
		mockApiFetch.mockImplementation((path: string) => {
			if (path.endsWith('/readme')) return Promise.reject(new ApiError('not found', 404));
			if (path.endsWith('/manifest')) return Promise.resolve(MANIFEST);
			if (path.endsWith('/dependencies')) return Promise.resolve({ namespace: NS, dependencies: [] });
			if (path.endsWith('/dependents')) return Promise.resolve({ namespace: NS, dependents: [] });
			if (path.includes(encodeURIComponent(OTHER_NS)))
				return Promise.resolve({ ...DETAIL, namespace: 'github.com/rabbyte/other', name: 'Other Arrow' });
			return Promise.resolve(DETAIL);
		});

		function Switcher() {
			const [namespace, setNamespace] = useState(NS);
			return (
				<>
					<button onClick={() => setNamespace(OTHER_NS)} type="button">
						switch arrow
					</button>
					<ArrowDetailsScreen namespace={namespace} />
				</>
			);
		}

		const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
		const rootRoute = createRootRoute({ component: Switcher });
		const router = createRouter({
			routeTree: rootRoute,
			history: createMemoryHistory({ initialEntries: ['/'] }),
		});
		render(
			<QueryClientProvider client={client}>
				<RouterProvider router={router} />
			</QueryClientProvider>
		);

		await user.click(await screen.findByRole('tab', { name: 'Methods' }));
		expect(screen.getByRole('tab', { name: 'Methods', selected: true })).toBeInTheDocument();

		await user.click(screen.getByRole('button', { name: 'switch arrow' }));
		expect(await screen.findByRole('heading', { name: 'Other Arrow' })).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Overview', selected: true })).toBeInTheDocument();
	});

	it('hides the Activity tab and any Settings in the rail for a Discovered (not-in-library) arrow', async () => {
		mockDetailAndManifest({ ...DETAIL, user_installed: false });
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		expect(screen.queryByRole('tab', { name: 'Activity' })).not.toBeInTheDocument();
		expect(screen.queryByText('1 setting')).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Add to Library' })).toBeInTheDocument();
	});

	it('shows no Settings in the rail for an installed arrow that declares no variables', async () => {
		mockDetailAndManifest(DETAIL, { ...MANIFEST, variables: [] });
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		expect(screen.queryByText('1 setting')).not.toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Activity' })).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Methods' })).toBeInTheDocument();
	});

	it('overlays live state from the reactive store on top of the fetched detail', async () => {
		mockDetailAndManifest();
		useArrowStore.setState({
			arrows: new Map([
				[
					NS,
					{
						namespace: NS,
						name: 'Minecraft Server',
						description: '',
						tags: [],
						icon: null,
						banner: null,
						version: '1.21.4',
						state: 'running',
						active_run: null,
						last_return: null,
					},
				],
			]),
		});
		renderScreen(NS);

		expect(await screen.findByText('Running')).toBeInTheDocument();
	});

	it('auto-switches to Activity when a live run starts, but does not yank back when it ends', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest();
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		expect(screen.getByRole('tab', { name: 'Overview', selected: true })).toBeInTheDocument();

		act(() => {
			useArrowStore.setState({
				arrows: new Map([
					[
						NS,
						{
							namespace: NS,
							name: 'Minecraft Server',
							description: '',
							tags: [],
							icon: null,
							banner: null,
							version: '1.21.4',
							state: 'running',
							active_run: { method: 'execute', variables: {}, steps: [] },
							last_return: null,
						},
					],
				]),
			});
		});
		expect(await screen.findByRole('tab', { name: 'Activity', selected: true })).toBeInTheDocument();

		await user.click(screen.getByRole('tab', { name: 'Overview' }));
		expect(screen.getByRole('tab', { name: 'Overview', selected: true })).toBeInTheDocument();

		// The run ends -- a deliberate navigation away from Activity should
		// not be reverted by the transition back to a non-running state.
		act(() => {
			useArrowStore.setState({
				arrows: new Map([
					[
						NS,
						{
							namespace: NS,
							name: 'Minecraft Server',
							description: '',
							tags: [],
							icon: null,
							banner: null,
							version: '1.21.4',
							state: 'ready',
							active_run: null,
							last_return: null,
						},
					],
				]),
			});
		});
		expect(screen.getByRole('tab', { name: 'Overview', selected: true })).toBeInTheDocument();
	});

	it('does not seed a value for a variable that declares no default', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest(DETAIL, {
			...MANIFEST,
			variables: [{ name: 'no-default', description: '', type: 'string' }],
		});
		renderScreen(NS);

		await user.click(await screen.findByRole('button', { name: 'Start' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(
				expect.stringContaining('/execute'),
				expect.objectContaining({ body: JSON.stringify({ variables: {} }) })
			)
		);
	});

	it('falls back to the first declared target when none matches the current platform at all', async () => {
		mockDetailAndManifest(DETAIL, {
			...MANIFEST,
			targets: {
				// Neither key is a platform any test environment reports (see the
				// comment on the default MANIFEST fixture above) -- this proves the
				// `?? detail.targets[0]` fallback, not the exact-match path.
				'darwin/arm64': MANIFEST.targets['darwin/arm64'],
				'windows/amd64': MANIFEST.targets['darwin/arm64'],
			},
		});
		renderScreen(NS);
		expect(await screen.findByText('4 cores')).toBeInTheDocument();
	});

	it('edits flow through the rail Settings panel into the same values used by run actions', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest();
		renderScreen(NS);

		const field = await screen.findByRole('textbox', { name: 'server-name' });
		await user.clear(field);
		await user.type(field, 'Renamed');

		await user.click(screen.getByRole('button', { name: 'Start' }));
		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith(
				expect.stringContaining('/execute'),
				expect.objectContaining({ body: JSON.stringify({ variables: { 'server-name': 'Renamed' } }) })
			)
		);
	});

	it('reuses the cached rich step detail when a live last_return reports the same run', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest({
			...DETAIL,
			last_return: {
				method: 'install',
				outcome: 'failed',
				variables: {},
				steps: [
					{ index: 0, title: 'Verify checksum', status: 'failed', type: 'run', error: 'checksum mismatch' },
				],
			},
		});
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });

		useArrowStore.setState({
			arrows: new Map([
				[
					NS,
					{
						namespace: NS,
						name: 'Minecraft Server',
						description: '',
						tags: [],
						icon: null,
						banner: null,
						version: '1.21.4',
						state: 'absent',
						active_run: null,
						last_return: { method: 'install', outcome: 'failed' },
					},
				],
			]),
		});

		await user.click(await screen.findByRole('button', { name: /Issue/ }));
		expect(await screen.findByText('checksum mismatch')).toBeInTheDocument();
	});

	it('does not count a bare (unversioned) store entry sharing the base namespace as an installed version', async () => {
		mockDetailAndManifest();
		useArrowStore.setState({
			arrows: new Map([
				[
					'github.com/rabbyte/minecraft',
					{
						namespace: 'github.com/rabbyte/minecraft',
						name: 'Minecraft Server',
						description: '',
						tags: [],
						icon: null,
						banner: null,
						version: '',
						state: 'absent',
						active_run: null,
						last_return: null,
					},
				],
			]),
		});
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		expect(screen.queryByRole('combobox', { name: 'Version' })).not.toBeInTheDocument();
	});

	it('renders with no methods when the arrow declares no targets at all', async () => {
		mockDetailAndManifest(DETAIL, { ...MANIFEST, targets: {} });
		const user = userEvent.setup();
		renderScreen(NS);

		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await user.click(screen.getByRole('tab', { name: 'Methods' }));
		expect(await screen.findByText('0 methods')).toBeInTheDocument();
	});

	it('falls back to an empty step list when a live last_return reports a different run than the cached rich one', async () => {
		mockDetailAndManifest({
			...DETAIL,
			last_return: {
				method: 'install',
				outcome: 'failed',
				variables: {},
				steps: [
					{ index: 0, title: 'Verify checksum', status: 'failed', type: 'run', error: 'checksum mismatch' },
				],
			},
		});
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });

		// The live store now reports a *different* completed run (uninstall,
		// not install) -- the cached install-run's step detail must not leak
		// into this new, unrelated outcome.
		useArrowStore.setState({
			arrows: new Map([
				[
					NS,
					{
						namespace: NS,
						name: 'Minecraft Server',
						description: '',
						tags: [],
						icon: null,
						banner: null,
						version: '1.21.4',
						state: 'absent',
						active_run: null,
						last_return: { method: 'uninstall', outcome: 'success' },
					},
				],
			]),
		});

		await waitFor(() => expect(screen.queryByText(/Issue/)).not.toBeInTheDocument());
	});

	it('navigates to the sibling version when a different one is picked from the Hero switcher', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest();
		useArrowStore.setState({
			arrows: new Map([
				[
					NS,
					{
						namespace: NS,
						name: 'Minecraft Server',
						description: '',
						tags: [],
						icon: null,
						banner: null,
						version: '1.21.4',
						state: 'ready',
						active_run: null,
						last_return: null,
					},
				],
				[
					'github.com/rabbyte/minecraft@v1.20.1',
					{
						namespace: 'github.com/rabbyte/minecraft@v1.20.1',
						name: 'Minecraft Server',
						description: '',
						tags: [],
						icon: null,
						banner: null,
						version: '1.20.1',
						state: 'outdated',
						active_run: null,
						last_return: null,
					},
				],
			]),
		});
		const { router } = renderScreen(NS);

		await user.click(await screen.findByRole('combobox', { name: 'Version' }));
		await user.click(await screen.findByRole('option', { name: 'v1.20.1' }));

		await waitFor(() =>
			expect(router.state.location.pathname).toBe('/arrow/github.com/rabbyte/minecraft%40v1.20.1')
		);
	});
});

describe('wide-screen tab grouping', () => {
	let restoreResizeObserver: () => void;

	beforeEach(() => {
		restoreResizeObserver = installMockResizeObserver();
	});

	afterEach(() => {
		restoreResizeObserver();
	});

	async function fireWidth(width: number) {
		const container = screen.getByTestId('arrow-tabs-container');
		await waitFor(() => expect(MockResizeObserver.for(container)).toBeDefined());
		act(() => {
			MockResizeObserver.for(container)!.fire(width);
		});
	}

	it('keeps every tab separate on a narrow container', async () => {
		mockDetailAndManifest();
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await fireWidth(500);

		expect(screen.getByRole('tab', { name: 'Overview' })).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Activity' })).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Methods' })).toBeInTheDocument();
	});

	it('merges Activity with Methods when wide, and keeps Overview on its own tab', async () => {
		const user = userEvent.setup();
		mockDetailAndManifest(DETAIL, MANIFEST, '## About');
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await fireWidth(1200);

		expect(await screen.findByRole('tab', { name: 'Overview' })).toBeInTheDocument();
		expect(screen.queryByRole('tab', { name: 'Activity' })).not.toBeInTheDocument();
		expect(screen.queryByRole('tab', { name: 'Methods' })).not.toBeInTheDocument();

		await user.click(screen.getByRole('tab', { name: 'Activity / Methods' }));
		expect(screen.getByText('No activity yet.')).toBeInTheDocument();
		expect(screen.getByText('backup')).toBeInTheDocument();
	});

	it('still merges Activity with Methods when Overview has no README', async () => {
		mockDetailAndManifest();
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await fireWidth(1200);

		expect(await screen.findByRole('tab', { name: 'Overview' })).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Activity / Methods' })).toBeInTheDocument();
	});

	it('never merges Overview even at a very wide width', async () => {
		mockDetailAndManifest();
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await fireWidth(4000);

		expect(await screen.findByRole('tab', { name: 'Overview' })).toBeInTheDocument();
	});

	it('leaves Methods on its own tab when wide and there is no Activity tab to pair it with', async () => {
		mockDetailAndManifest({ ...DETAIL, user_installed: false });
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await fireWidth(1200);

		expect(await screen.findByRole('tab', { name: 'Methods' })).toBeInTheDocument();
		expect(screen.queryByRole('tab', { name: 'Activity / Methods' })).not.toBeInTheDocument();
	});
});

describe('wide-screen details rail', () => {
	let restoreResizeObserver: () => void;

	beforeEach(() => {
		restoreResizeObserver = installMockResizeObserver();
	});

	afterEach(() => {
		restoreResizeObserver();
	});

	async function fireWidth(width: number) {
		const container = screen.getByTestId('arrow-detail-content');
		await waitFor(() => expect(MockResizeObserver.for(container)).toBeDefined());
		act(() => {
			MockResizeObserver.for(container)!.fire(width);
		});
	}

	it('stacks the Details rail under the tabs on a narrow container', async () => {
		mockDetailAndManifest();
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await fireWidth(600);

		expect(screen.getByTestId('arrow-detail-layout')).toHaveClass('flex-col');
	});

	it('sits the Details rail beside the tabs once the container is wide enough', async () => {
		mockDetailAndManifest();
		renderScreen(NS);
		await screen.findByRole('heading', { name: 'Minecraft Server' });
		await fireWidth(1000);

		expect(screen.getByTestId('arrow-detail-layout')).toHaveClass('grid');
	});
});
