import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider, createMemoryHistory, createRouter } from '@tanstack/react-router';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(() => Promise.resolve()) }));

vi.mock('@tanstack/react-router-devtools', () => ({ TanStackRouterDevtools: () => null }));

import { MockIndicator } from '@/components/mock-indicator';

import { QUIVER_CORE_NAMESPACE } from '@/domain/release';
import { useCatalogVisibilityStore } from '@/features/settings/stores/catalog-visibility-store';
import { useThemeStore } from '@/features/shell';
import { useShellStore } from '@/features/shell/stores/shell-store';
import { useArrowStore } from '@/lib/core-store';
import { toRuntimeUpdate } from '@/lib/core-store/dtos/v0/runtime';
import { LOCALE_STORAGE_KEY, useLocaleStore } from '@/lib/i18n';
import { createMockBackend, currentMock, disposeMock, installMock } from '@/lib/mock';
import { setMockCorrected } from '@/lib/mock/server/handlers/config';
import { FAULT_KEYS, useMockStore } from '@/lib/mock/store';
import { arrow } from '@/lib/mock/world/scenarios/kit';
import { ApiError } from '@/lib/transport/api';
import { installBackend, resetBackend } from '@/lib/transport/backend';
import { routeTree } from '@/routeTree.gen';

import * as engineApi from './api/engine-api';
import * as pathApi from './api/path-api';
import { DeveloperSettings } from './components/tabs/developer';
import { EngineSettings } from './components/tabs/engine';
import { GeneralSettings } from './components/tabs/general';
import { useEngineStore } from './stores/engine-store';
import { usePathStore } from './stores/path-store';
import { useSettingsUI } from './stores/settings-store';

let reload: ReturnType<typeof vi.fn>;

beforeEach(() => {
	vi.clearAllMocks();
	reload = vi.fn();
	Object.defineProperty(window, 'location', { value: { ...window.location, reload }, writable: true });

	useSettingsUI.setState({ tab: 'general' });
	useMockStore.setState({ enabled: false, scenario: 'normal', latency: 0, errorRate: 0, unreachable: false });
	useMockStore.getState().resetFaults();
	useLocaleStore.setState({ preference: 'system', detected: 'en' });
	localStorage.removeItem(LOCALE_STORAGE_KEY);
	useThemeStore.setState({ preference: 'system' });
	useShellStore.setState({ sidebarSide: 'left' });
	useCatalogVisibilityStore.setState({ showSelfComponents: true });
});

afterEach(() => {
	disposeMock();
	resetBackend();
});

function renderEngine() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return render(
		<QueryClientProvider client={client}>
			<EngineSettings />
		</QueryClientProvider>
	);
}

function renderApp(path: string) {
	const router = createRouter({ routeTree, history: createMemoryHistory({ initialEntries: [path] }) });
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	render(
		<QueryClientProvider client={client}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	);
	return router;
}

describe('the Developer panel', () => {
	it('reloads when the mock switch is flipped, because the backend is chosen at boot', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		await user.click(screen.getByRole('switch', { name: 'Use the mock server' }));

		expect(useMockStore.getState().enabled).toBe(true);
		expect(reload).toHaveBeenCalled();
	});

	it('does not reload until Apply is pressed', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled();
		await user.click(screen.getByRole('combobox', { name: 'Mock scenario' }));
		await user.click(await screen.findByRole('option', { name: 'Extreme' }));

		expect(reload).not.toHaveBeenCalled();
		expect(useMockStore.getState().scenario).toBe('normal');

		await user.click(screen.getByRole('button', { name: 'Apply' }));
		expect(useMockStore.getState().scenario).toBe('extreme');
		expect(reload).toHaveBeenCalled();
	});

	it('shows the picked scenario’s summary, not its slug', async () => {
		render(<DeveloperSettings />);
		expect(screen.getByText(/every state · a failed install/)).toBeInTheDocument();
	});

	it('drives latency, error rate and the unreachable switch into the store', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		const latency = screen.getByRole('textbox', { name: 'Latency in milliseconds' });
		await user.clear(latency);
		await user.type(latency, '250');
		await user.tab();
		expect(useMockStore.getState().latency).toBe(250);

		const errorRate = screen.getByRole('textbox', { name: 'Error rate percentage' });
		await user.clear(errorRate);
		await user.type(errorRate, '25');
		await user.tab();
		expect(useMockStore.getState().errorRate).toBe(25);

		await user.click(screen.getByRole('switch', { name: 'Daemon unreachable' }));
		expect(useMockStore.getState().unreachable).toBe(true);
	});

	it('offers a per-row reset for the error rate and the unreachable switch too', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		expect(screen.getByRole('button', { name: 'Reset Error rate' })).toBeDisabled();
		expect(screen.getByRole('button', { name: 'Reset Daemon unreachable' })).toBeDisabled();

		useMockStore.setState({ errorRate: 25 });
		await user.click(await screen.findByRole('button', { name: 'Reset Error rate' }));
		expect(useMockStore.getState().errorRate).toBe(0);

		await user.click(screen.getByRole('switch', { name: 'Daemon unreachable' }));
		const reset = await screen.findByRole('button', { name: 'Reset Daemon unreachable' });
		expect(reset).toBeEnabled();
		await user.click(reset);
		expect(useMockStore.getState().unreachable).toBe(false);
	});

	it('stores a slider change as a number, not the array Base UI can emit', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		screen.getByLabelText('Search fault rate').focus();
		await user.keyboard('{ArrowRight}');

		expect(useMockStore.getState().faults.search).toBe(5);
		expect(Number.isNaN(useMockStore.getState().faults.search)).toBe(false);
	});

	it('lists one slider per route family, each with its own reset', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		expect(screen.getAllByRole('slider', { hidden: true })).toHaveLength(FAULT_KEYS.length);
		expect(screen.getByRole('button', { name: 'Reset Search' })).toBeDisabled();

		useMockStore.getState().setFault('search', 40);
		render(<DeveloperSettings />);
		const resets = screen.getAllByRole('button', { name: 'Reset Search' });
		await user.click(resets[resets.length - 1]);
		expect(useMockStore.getState().faults.search).toBe(0);
	});

	it('disables the switch and says why when the environment forced it on', () => {
		vi.stubEnv('VITE_QUIVER_MOCK', '1');
		render(<DeveloperSettings />);

		const sw = screen.getByRole('switch', { name: 'Use the mock server' });
		expect(sw).toHaveAttribute('aria-disabled', 'true');
		expect(sw).toBeChecked();
		expect(screen.getByText(/Forced on by VITE_QUIVER_MOCK/)).toBeInTheDocument();
		expect(screen.queryByText(/do nothing while the mock server is off/i)).not.toBeInTheDocument();
		vi.unstubAllEnvs();
	});
});

describe('the Developer panel’s inert-controls notice', () => {
	it('warns that chaos and faults are inert while the mock server is off', () => {
		render(<DeveloperSettings />);
		expect(screen.getByText(/do nothing while the mock server is off/i)).toBeInTheDocument();
	});

	it('goes away once the mock server is switched on', () => {
		useMockStore.setState({ enabled: true });
		render(<DeveloperSettings />);
		expect(screen.queryByText(/do nothing while the mock server is off/i)).not.toBeInTheDocument();
	});
});

describe('the Developer panel resets', () => {
	it('offers a per-row reset once a chaos value is off its default', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);
		expect(screen.getByRole('button', { name: 'Reset Latency' })).toBeDisabled();
		useMockStore.setState({ latency: 250 });
		const reset = await screen.findByRole('button', { name: 'Reset Latency' });
		expect(reset).toBeEnabled();
		await user.click(reset);
		expect(useMockStore.getState().latency).toBe(0);
	});

	it('no longer offers the old reset-everything buttons', () => {
		render(<DeveloperSettings />);
		expect(screen.queryByRole('button', { name: /reset chaos/i })).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /reset all faults/i })).not.toBeInTheDocument();
	});
});

describe('the settings page', () => {
	it('renders its panels on arrival, with nothing to open first', async () => {
		renderApp('/settings');
		expect(await screen.findByRole('tab', { name: 'General' })).toBeInTheDocument();
	});

	it('lands on the tab the URL names', async () => {
		renderApp('/settings?tab=developer');
		expect(await screen.findByRole('tab', { name: 'Developer' })).toHaveAttribute('aria-selected', 'true');
	});

	it('writes the picked tab back into the URL', async () => {
		const user = userEvent.setup();
		const router = renderApp('/settings');

		await user.click(await screen.findByRole('tab', { name: 'Developer' }));
		await waitFor(() => expect(router.state.location.searchStr).toBe('?tab=developer'));
	});

	it('falls back to the first tab when the remembered one is gone', async () => {
		useSettingsUI.setState({ tab: 'developer' });
		renderApp('/settings');

		const tabs = await screen.findAllByRole('tab');
		expect(tabs.some((tab) => tab.getAttribute('aria-selected') === 'true')).toBe(true);
	});
});

describe('the mock indicator', () => {
	it('renders nothing when no mock is installed', () => {
		const { container } = render(<MockIndicator />);
		expect(container).toBeEmptyDOMElement();
	});

	it('names the live scenario once a mock is actually installed', async () => {
		installMock('extreme');
		expect(currentMock()).not.toBeNull();

		renderApp('/');
		expect(await screen.findByText('Mock')).toBeInTheDocument();
		expect(screen.getByText(/Extreme · no daemon is being contacted/)).toBeInTheDocument();
	});

	it('opens the Developer tab from its own Turn off link', async () => {
		const user = userEvent.setup();
		installMock('normal');
		renderApp('/');

		await user.click(await screen.findByRole('link', { name: 'Turn off' }));
		expect(await screen.findByRole('tab', { name: 'Developer' })).toHaveAttribute('aria-selected', 'true');
	});

	it('stays silent when the store says enabled but no backend was installed', () => {
		useMockStore.setState({ enabled: true });
		const { container } = render(<MockIndicator />);
		expect(container).toBeEmptyDOMElement();
	});
});

describe('installMock', () => {
	it('survives a fixture that throws, leaving the real backend in place', () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		const boom = vi.fn(() => {
			throw new Error('bad fixture');
		});
		const spy = vi.spyOn(Map.prototype, 'set').mockImplementationOnce(boom as never);

		const result = installMock('normal');
		spy.mockRestore();

		expect(result).toBeNull();
		expect(currentMock()).toBeNull();
	});

	it('disposes a previously installed mock before replacing it', () => {
		const first = installMock('normal')!;
		const disposeSpy = vi.spyOn(first, 'dispose');
		const second = installMock('empty')!;

		expect(second).not.toBe(first);
		expect(second.world.scenario).toBe('empty');
		expect(disposeSpy).toHaveBeenCalled();
		disposeSpy.mockRestore();
	});
});

describe('a stand-in backend can be installed over the mock', () => {
	it('takes effect for the next call', async () => {
		const runtime = createMockBackend('normal');
		installBackend(runtime.backend);
		const { connections } = await runtime.backend.getConnections();
		expect(connections[0].id).toBe('mock:normal');
		runtime.dispose();
	});
});

describe('the General panel', () => {
	it('offers "follow the system" first, and names the system language in it', () => {
		render(<GeneralSettings />);
		expect(screen.getByRole('combobox', { name: 'Display language' })).toHaveTextContent('System (English)');
	});

	it('lists every shipped language under its own name', async () => {
		const user = userEvent.setup();
		render(<GeneralSettings />);

		await user.click(screen.getByRole('combobox', { name: 'Display language' }));
		expect(await screen.findByRole('option', { name: 'English' })).toBeInTheDocument();
	});

	it('writes the choice to the persisted preference', async () => {
		const user = userEvent.setup();
		render(<GeneralSettings />);

		await user.click(screen.getByRole('combobox', { name: 'Display language' }));
		await user.click(await screen.findByRole('option', { name: 'English' }));

		expect(useLocaleStore.getState().preference).toBe('en');
		const persisted = JSON.parse(localStorage.getItem(LOCALE_STORAGE_KEY) ?? '{}') as {
			state?: { preference?: string };
		};
		expect(persisted.state?.preference).toBe('en');
	});

	it('disables the picker and says why when the environment forced the locale', () => {
		vi.stubEnv('VITE_QUIVER_LOCALE', 'en');
		render(<GeneralSettings />);

		expect(screen.getByRole('combobox', { name: 'Display language' })).toBeDisabled();
		expect(screen.getByText(/Forced to English by VITE_QUIVER_LOCALE/)).toBeInTheDocument();
		vi.unstubAllEnvs();
	});

	it('drives the theme preference', async () => {
		const user = userEvent.setup();
		render(<GeneralSettings />);
		await user.click(screen.getByLabelText('Theme'));
		await user.click(await screen.findByRole('option', { name: 'Dark' }));
		expect(useThemeStore.getState().preference).toBe('dark');
	});

	it('drives the sidebar side', async () => {
		const user = userEvent.setup();
		render(<GeneralSettings />);
		await user.click(screen.getByLabelText('Sidebar side'));
		await user.click(await screen.findByRole('option', { name: 'Right' }));
		expect(useShellStore.getState().sidebarSide).toBe('right');
	});

	it('resets the sidebar side back to left', async () => {
		const user = userEvent.setup();
		useShellStore.setState({ sidebarSide: 'right' });
		render(<GeneralSettings />);
		await user.click(screen.getByRole('button', { name: 'Reset Sidebar side' }));
		expect(useShellStore.getState().sidebarSide).toBe('left');
	});

	it('resets the language preference back to system', async () => {
		const user = userEvent.setup();
		useLocaleStore.setState({ preference: 'en', detected: 'en' });
		render(<GeneralSettings />);
		await user.click(screen.getByRole('button', { name: 'Reset Display language' }));
		expect(useLocaleStore.getState().preference).toBe('system');
	});

	it('offers a reset only once the value differs from the default', async () => {
		const user = userEvent.setup();
		render(<GeneralSettings />);
		expect(screen.getByRole('button', { name: 'Reset Theme' })).toBeDisabled();
		await user.click(screen.getByLabelText('Theme'));
		await user.click(await screen.findByRole('option', { name: 'Dark' }));
		const reset = screen.getByRole('button', { name: 'Reset Theme' });
		expect(reset).toBeEnabled();
		await user.click(reset);
		expect(useThemeStore.getState().preference).toBe('system');
	});

	it('shows Quiver’s own components by default -- opt-out, not opt-in', () => {
		render(<GeneralSettings />);
		expect(screen.getByRole('switch', { name: "Show Quiver's own components" })).toBeChecked();
		expect(screen.getByRole('button', { name: "Reset Show Quiver's own components" })).toBeDisabled();
	});

	it('turns off Quiver’s own components in listings', async () => {
		const user = userEvent.setup();
		render(<GeneralSettings />);

		await user.click(screen.getByRole('switch', { name: "Show Quiver's own components" }));

		expect(useCatalogVisibilityStore.getState().showSelfComponents).toBe(false);
		expect(screen.getByRole('switch', { name: "Show Quiver's own components" })).not.toBeChecked();
	});

	it('resets the self-components toggle back to shown', async () => {
		const user = userEvent.setup();
		useCatalogVisibilityStore.setState({ showSelfComponents: false });
		render(<GeneralSettings />);

		const reset = screen.getByRole('button', { name: "Reset Show Quiver's own components" });
		expect(reset).toBeEnabled();
		await user.click(reset);

		expect(useCatalogVisibilityStore.getState().showSelfComponents).toBe(true);
	});
});

describe('the Engine panel', () => {
	beforeEach(() => {
		installMock('normal');
		useEngineStore.setState({ view: null, rejected: [], loading: true, error: null, patchError: null });
	});

	it('shows the daemon values once loaded', async () => {
		renderEngine();
		expect(await screen.findByDisplayValue('49152')).toBeInTheDocument();
		expect(screen.getByDisplayValue('65535')).toBeInTheDocument();
	});

	it('says nothing about restarting until something is pending', async () => {
		renderEngine();
		await screen.findByDisplayValue('49152');
		expect(screen.queryByText(/restart/i)).not.toBeInTheDocument();
	});

	it('announces the restart once a change is pending', async () => {
		const user = userEvent.setup();
		renderEngine();
		const start = await screen.findByDisplayValue('49152');
		await user.clear(start);
		await user.type(start, '27015');
		await user.tab();
		expect(await screen.findByText(/restart/i)).toBeInTheDocument();
	});

	it('shows the daemon default again after a reset', async () => {
		const user = userEvent.setup();
		renderEngine();
		const start = await screen.findByDisplayValue('49152');
		await user.clear(start);
		await user.type(start, '27015');
		await user.tab();
		await screen.findByDisplayValue('27015');
		await user.click(screen.getByRole('button', { name: 'Reset Ports for servers' }));
		expect(await screen.findByDisplayValue('49152')).toBeInTheDocument();
	});

	it('reports settings the daemon had to replace with defaults, including why', async () => {
		setMockCorrected(currentMock()!.world, ['vault.ttl']);
		renderEngine();
		expect(
			await screen.findByText(/could not use these settings.*vault\.ttl \(unusable value, default applied\)/i)
		).toBeInTheDocument();
	});

	it('marks the row the daemon refused, attributed to the lowest port', async () => {
		const user = userEvent.setup();
		renderEngine();
		const start = await screen.findByDisplayValue('49152');
		await user.clear(start);
		await user.type(start, '99999');
		await user.tab();
		expect(await screen.findByText('Lowest port: port out of range')).toBeInTheDocument();
	});

	it('attributes a rejection on the highest port to that field, not the lowest', async () => {
		const user = userEvent.setup();
		renderEngine();
		const end = await screen.findByDisplayValue('65535');
		await user.clear(end);
		await user.type(end, '99999');
		await user.tab();
		expect(await screen.findByText('Highest port: port out of range')).toBeInTheDocument();
		expect(screen.queryByText(/^Lowest port:/)).not.toBeInTheDocument();
	});

	it('reverts a rejected port to the daemon value, while still showing why', async () => {
		const user = userEvent.setup();
		renderEngine();
		const start = await screen.findByDisplayValue('49152');
		await user.clear(start);
		await user.type(start, '99999');
		await user.tab();

		// The rejection message and the reverted field must both be true at
		// once — a refused value should never look accepted just because the
		// input still shows it.
		expect(await screen.findByDisplayValue('49152')).toBeInTheDocument();
		expect(screen.getByText(/port out of range/i)).toBeInTheDocument();
	});

	it('shows an error instead of the panel when the daemon cannot be reached', async () => {
		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 100 } });
		renderEngine();
		expect(await screen.findByText(/mock fault: config/i)).toBeInTheDocument();
		expect(screen.queryByDisplayValue('49152')).not.toBeInTheDocument();
	});

	it('offers a retry once the daemon cannot be reached, and recovers once it can', async () => {
		const user = userEvent.setup();
		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 100 } });
		renderEngine();
		const retry = await screen.findByRole('button', { name: 'Try again' });

		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 0 } });
		await user.click(retry);

		expect(await screen.findByDisplayValue('49152')).toBeInTheDocument();
	});

	it('drives the write-to-disk switch and resets it', async () => {
		const user = userEvent.setup();
		renderEngine();
		await screen.findByDisplayValue('49152');

		const reset = screen.getByRole('button', { name: 'Reset Write logs to disk' });
		expect(reset).toBeDisabled();

		await user.click(screen.getByRole('switch', { name: 'Write logs to disk' }));
		await waitFor(() => expect(screen.getByRole('switch', { name: 'Write logs to disk' })).not.toBeChecked());
		expect(screen.getByRole('button', { name: 'Reset Write logs to disk' })).toBeEnabled();

		await user.click(screen.getByRole('button', { name: 'Reset Write logs to disk' }));
		await waitFor(() => expect(screen.getByRole('switch', { name: 'Write logs to disk' })).toBeChecked());
	});

	it('drives the log level select and resets it', async () => {
		const user = userEvent.setup();
		renderEngine();
		await screen.findByDisplayValue('49152');

		await user.click(screen.getByLabelText('Level'));
		await user.click(await screen.findByRole('option', { name: 'Debug' }));
		expect(await screen.findByRole('button', { name: 'Reset Level' })).toBeEnabled();

		await user.click(screen.getByRole('button', { name: 'Reset Level' }));
		await waitFor(() => expect(useEngineStore.getState().view?.configured.logger.level).toBe('info'));
	});

	it('marks the log level row the daemon refused', async () => {
		renderEngine();
		await screen.findByDisplayValue('49152');

		// The Select only ever offers values the mock accepts, so a rejected
		// `logger.level` can only be reached by patching the store directly —
		// there is no user interaction that produces one.
		await act(async () => {
			await useEngineStore.getState().patch({ logger: { level: 'nonsense' } });
		});
		expect(await screen.findByText(/unusable log level/i)).toBeInTheDocument();
	});

	it('leaves a non-integer port untouched, without contacting the daemon', async () => {
		const user = userEvent.setup();
		renderEngine();
		const start = await screen.findByDisplayValue('49152');
		await user.clear(start);
		await user.type(start, '1.5');
		await user.tab();

		expect(screen.queryByText(/restart/i)).not.toBeInTheDocument();
		expect(useEngineStore.getState().view?.configured.netbridge.ephemeral_port_start).toBe(49152);
	});

	it('sends nothing when a port field is left empty, and it snaps back to the daemon value', async () => {
		const user = userEvent.setup();
		renderEngine();
		const start = await screen.findByDisplayValue('49152');
		await user.clear(start);
		await user.tab();

		// `Number('')` is `0`, which passes `Number.isInteger` — this must
		// fail if the empty-string case is not guarded against on its own.
		expect(await screen.findByDisplayValue('49152')).toBeInTheDocument();
		expect(screen.queryByText(/port out of range/i)).not.toBeInTheDocument();
		expect(currentMock()!.world.config.configured.netbridge.ephemeral_port_start).toBe(49152);
	});

	it('resets the high end of the port range on its own', async () => {
		const user = userEvent.setup();
		renderEngine();
		const end = await screen.findByDisplayValue('65535');
		await user.clear(end);
		await user.type(end, '60000');
		await user.tab();
		await screen.findByDisplayValue('60000');
		// The daemon must have actually accepted it, not just the on-screen
		// text — Reset would restore the default either way, so that alone
		// would not prove the high end was ever really patched.
		await waitFor(() => expect(currentMock()!.world.config.configured.netbridge.ephemeral_port_end).toBe(60000));

		await user.click(screen.getByRole('button', { name: 'Reset Ports for servers' }));
		expect(await screen.findByDisplayValue('65535')).toBeInTheDocument();
	});

	it('shows a loading state before the first view arrives, instead of an empty panel', async () => {
		renderEngine();
		const status = screen.getByRole('status', { name: 'Loading engine settings' });
		expect(status).toHaveAttribute('aria-busy', 'true');
		expect(screen.queryByDisplayValue('49152')).not.toBeInTheDocument();

		// Drains the in-flight load before the test ends, so nothing settles
		// and updates state after this test's render has gone away.
		await screen.findByDisplayValue('49152');
	});

	it('clears a stale rejection when the panel is left and revisited', async () => {
		const user = userEvent.setup();
		const { unmount } = renderEngine();

		const start = await screen.findByDisplayValue('49152');
		await user.clear(start);
		await user.type(start, '99999');
		await user.tab();
		expect(await screen.findByText('Lowest port: port out of range')).toBeInTheDocument();

		// Also leave a stale *patch* error behind — `rejected` and
		// `patchError` are two independent fields on the store, and a fix
		// that only clears one of them would still leave a red banner
		// sitting over an otherwise healthy panel after the trip below.
		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 100 } });
		await user.click(screen.getByRole('switch', { name: 'Write logs to disk' }));
		expect(await screen.findByText(/mock fault: config/i)).toBeInTheDocument();
		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 0 } });

		// Simulates `Tabs.Panel` unmounting the Engine tab (leaving General)
		// and remounting it (returning to Engine). The daemon still holds a
		// valid value — neither the earlier rejection nor the patch error
		// must survive the trip.
		unmount();
		renderEngine();

		await screen.findByDisplayValue('49152');
		expect(screen.queryByText(/port out of range/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/mock fault: config/i)).not.toBeInTheDocument();
	});

	it('keeps the panel and its rows on screen when a patch fails, showing the error inline', async () => {
		const user = userEvent.setup();
		renderEngine();
		await screen.findByDisplayValue('49152');

		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 100 } });
		await user.click(screen.getByRole('switch', { name: 'Write logs to disk' }));

		expect(await screen.findByText(/mock fault: config/i)).toBeInTheDocument();
		// The panel itself must still be there — a patch failure is not a
		// load failure, and must not tear the whole thing down.
		expect(screen.getByDisplayValue('49152')).toBeInTheDocument();
		expect(screen.getByRole('switch', { name: 'Write logs to disk' })).toBeInTheDocument();
	});

	it('normalises an on-disk log level alias for display', async () => {
		renderEngine();
		await screen.findByDisplayValue('49152');

		// The mock accepts `warning` as a valid on-disk alias (core does too;
		// the Select just never writes it). This is the only way to get one
		// onto `configured.logger.level` without reaching into the store.
		await act(async () => {
			await useEngineStore.getState().patch({ logger: { level: 'warning' } });
		});

		expect(useEngineStore.getState().view?.configured.logger.level).toBe('warning');
		expect(screen.getByRole('combobox', { name: 'Level' })).toHaveTextContent('Warn');
	});
});

describe("the Engine panel's auto-register row", () => {
	const LABEL = 'Auto-register arrows from repositories without an ARROW.md';

	beforeEach(() => {
		installMock('normal');
		useEngineStore.setState({ view: null, rejected: [], loading: true, error: null, patchError: null });
	});

	it('starts off, matching the daemon default', async () => {
		renderEngine();
		expect(await screen.findByRole('switch', { name: LABEL })).not.toBeChecked();
		expect(screen.getByRole('button', { name: `Reset ${LABEL}` })).toBeDisabled();
	});

	it('patches Fletcher on, leaves search alone, and says the daemon must restart', async () => {
		const user = userEvent.setup();
		renderEngine();
		await user.click(await screen.findByRole('switch', { name: LABEL }));

		await waitFor(() => expect(screen.getByRole('switch', { name: LABEL })).toBeChecked());
		const { configured, running } = currentMock()!.world.config;
		expect(configured.manifold.fletcher).toEqual({ enabled: true });
		expect(configured.search).toEqual(running.search);
		expect(running.manifold.fletcher).toEqual({ enabled: false });
		expect(screen.getAllByText(/restart/i).length).toBeGreaterThan(0);
		expect(screen.getByRole('button', { name: `Reset ${LABEL}` })).toBeEnabled();
	});

	it('patches Fletcher off again and drops the restart note', async () => {
		const user = userEvent.setup();
		renderEngine();
		await user.click(await screen.findByRole('switch', { name: LABEL }));
		await waitFor(() => expect(screen.getByRole('switch', { name: LABEL })).toBeChecked());

		await user.click(screen.getByRole('switch', { name: LABEL }));
		await waitFor(() => expect(screen.getByRole('switch', { name: LABEL })).not.toBeChecked());
		const { configured } = currentMock()!.world.config;
		expect(configured.manifold.fletcher).toEqual({ enabled: false });
		expect(screen.queryByText(/restart/i)).not.toBeInTheDocument();
	});

	it('resets the switch to the daemon default', async () => {
		const user = userEvent.setup();
		renderEngine();
		await user.click(await screen.findByRole('switch', { name: LABEL }));
		await waitFor(() => expect(screen.getByRole('switch', { name: LABEL })).toBeChecked());

		await user.click(screen.getByRole('button', { name: `Reset ${LABEL}` }));
		await waitFor(() => expect(screen.getByRole('switch', { name: LABEL })).not.toBeChecked());
	});

	it('is on when Fletcher is already on in the daemon, with nothing pending', async () => {
		const { configured, running } = currentMock()!.world.config;
		for (const doc of [configured, running]) doc.manifold.fletcher = { enabled: true };
		renderEngine();
		expect(await screen.findByRole('switch', { name: LABEL })).toBeChecked();
		expect(screen.queryByText(/restart/i)).not.toBeInTheDocument();
	});

	it('reads as off, without crashing, on a daemon that predates the keys', async () => {
		const { configured, running } = currentMock()!.world.config;
		for (const doc of [configured, running]) delete doc.manifold.fletcher;
		renderEngine();
		expect(await screen.findByRole('switch', { name: LABEL })).not.toBeChecked();
		expect(screen.getByDisplayValue('49152')).toBeInTheDocument();
	});

	it('shows the daemon’s refusal on the row when it does not know the keys', async () => {
		const user = userEvent.setup();
		renderEngine();
		const toggle = await screen.findByRole('switch', { name: LABEL });
		vi.spyOn(engineApi, 'patchConfig').mockResolvedValueOnce({
			applied: [],
			rejected: [{ key: 'manifold', message: 'unknown setting "manifold.fletcher"' }],
		});

		await user.click(toggle);

		expect(await screen.findByText('unknown setting "manifold.fletcher"')).toBeInTheDocument();
		expect(screen.getByRole('switch', { name: LABEL })).not.toBeChecked();
	});

	it('reports a failed patch inline and keeps the panel', async () => {
		const user = userEvent.setup();
		renderEngine();
		const toggle = await screen.findByRole('switch', { name: LABEL });
		useMockStore.setState({ faults: { ...useMockStore.getState().faults, config: 100 } });

		await user.click(toggle);

		expect(await screen.findByText(/mock fault: config/i)).toBeInTheDocument();
		expect(screen.getByRole('switch', { name: LABEL })).toBeInTheDocument();
	});
});

describe("the Engine panel's self-update channel row", () => {
	beforeEach(() => {
		installMock('normal');
		useEngineStore.setState({ view: null, rejected: [], loading: true, error: null, patchError: null });
	});

	it('disables the picker and explains why when quiver.core has not self-registered yet', async () => {
		renderEngine();
		await screen.findByDisplayValue('49152');

		expect(await screen.findByText("Couldn't load quiver.core's channels")).toBeInTheDocument();
		expect(screen.getByRole('combobox', { name: 'Update channel' })).toBeDisabled();
	});

	it('offers quiver.core’s own published channels once its self-registration exists', async () => {
		const user = userEvent.setup();
		currentMock()!.world.arrows.set(
			`${QUIVER_CORE_NAMESPACE}@stable-1.0`,
			arrow({
				namespace: QUIVER_CORE_NAMESPACE,
				name: 'Quiver Core',
				state: 'ready',
				ref: 'stable-1.0',
				version: '1.0',
				channels: [
					{
						name: 'stable',
						kind: 'ordered',
						latest: 'stable-1.1',
						count: 2,
						members: ['stable-1.1', 'stable-1.0'],
					},
					{ name: 'nightly', kind: 'pointer', latest: 'nightly-latest' },
				],
			})
		);

		renderEngine();
		await screen.findByDisplayValue('49152');

		const select = await screen.findByRole('combobox', { name: 'Update channel' });
		expect(select).toBeEnabled();
		await user.click(select);
		expect(await screen.findByRole('option', { name: 'stable' })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: 'nightly' })).toBeInTheDocument();
	});

	it('patches arrows.self_update_channel when a channel is picked, and resets it back', async () => {
		const user = userEvent.setup();
		currentMock()!.world.arrows.set(
			`${QUIVER_CORE_NAMESPACE}@stable-1.0`,
			arrow({
				namespace: QUIVER_CORE_NAMESPACE,
				name: 'Quiver Core',
				state: 'ready',
				ref: 'stable-1.0',
				version: '1.0',
				channels: [{ name: 'beta', kind: 'ordered', latest: 'beta-1.1', count: 1, members: ['beta-1.1'] }],
			})
		);

		renderEngine();
		await screen.findByDisplayValue('49152');

		const select = await screen.findByRole('combobox', { name: 'Update channel' });
		await user.click(select);
		await user.click(await screen.findByRole('option', { name: 'beta' }));

		await waitFor(() =>
			expect(useEngineStore.getState().view?.configured.arrows?.self_update_channel).toBe('beta')
		);
		expect(await screen.findByRole('button', { name: 'Reset Update channel' })).toBeEnabled();

		await user.click(screen.getByRole('button', { name: 'Reset Update channel' }));
		await waitFor(() => expect(useEngineStore.getState().view?.configured.arrows?.self_update_channel).toBe(''));
	});
});

describe("the Engine panel's update rows", () => {
	const CORE = `${QUIVER_CORE_NAMESPACE}@stable-26.5`;
	const AHEAD = { ref: 'stable-26.6', commit: 'abc' };
	const STAGED = { version: 'stable-26.6', staged_at: '2026-10-03T10:00:00Z' };

	function registerCore(overrides: Partial<ReturnType<typeof arrow>> = {}) {
		currentMock()!.world.arrows.set(
			CORE,
			arrow({
				namespace: QUIVER_CORE_NAMESPACE,
				name: 'Quiver Core',
				ref: 'stable-26.5',
				version: '26.5',
				state: 'ready',
				stages_update: true,
				...overrides,
			})
		);
		useArrowStore.getState().reset();
		useArrowStore.getState().setCatalog([
			{
				connectionId: 'local',
				namespace: CORE,
				name: 'Quiver Core',
				description: '',
				tags: [],
				icon: null,
				banner: null,
				version: 'stable-26.5',
			},
		]);
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: CORE,
			state: overrides.state ?? 'ready',
			active_run: overrides.active_run ?? null,
			last_return: null,
		});
	}

	beforeEach(() => {
		installMock('normal');
		useEngineStore.setState({ view: null, rejected: [], loading: true, error: null, patchError: null });
		useArrowStore.getState().reset();
		// What the app's runtime listener does with each frame the daemon pushes.
		currentMock()!.backend.openSocket('/v0/runtime').onmessage = (e) =>
			useArrowStore.getState().applyRuntimeUpdate(toRuntimeUpdate(JSON.parse(e.data)));
	});

	it('says plainly that it has no version to show before quiver.core has registered itself', async () => {
		renderEngine();
		await screen.findByDisplayValue('49152');

		expect(await screen.findByText("Version information isn't available yet.")).toBeInTheDocument();
		expect(screen.getByText('Unknown')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Check for updates' })).toBeDisabled();
	});

	it('shows the installed version and that it is up to date, offering only a check', async () => {
		registerCore();
		renderEngine();

		expect(await screen.findByText('stable-26.5')).toBeInTheDocument();
		expect(await screen.findByText('quiver.core is up to date.')).toBeInTheDocument();
		expect(screen.getByText('None')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Check for updates' })).toBeEnabled();
		expect(screen.queryByRole('button', { name: 'Update' })).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Restart to apply' })).not.toBeInTheDocument();
	});

	it('asks core to check now when Check for updates is pressed', async () => {
		const user = userEvent.setup();
		registerCore();
		renderEngine();
		await screen.findByText('quiver.core is up to date.');
		const fetch = vi.spyOn(currentMock()!.backend, 'fetch');

		await user.click(screen.getByRole('button', { name: 'Check for updates' }));

		await waitFor(() =>
			expect(fetch).toHaveBeenCalledWith(
				`/v0/arrow/${encodeURIComponent(CORE)}`,
				expect.objectContaining({ method: 'PATCH' })
			)
		);
	});

	it('shows the available version and offers Update', async () => {
		registerCore({ state: 'outdated', available: AHEAD });
		renderEngine();

		expect(await screen.findByText('Version stable-26.6 is available.')).toBeInTheDocument();
		expect(screen.getAllByText('stable-26.6').length).toBeGreaterThan(0);
		expect(screen.getByRole('button', { name: 'Update' })).toBeEnabled();
	});

	it('starts the update without any variables and shows it downloading, leaving a second press out of reach', async () => {
		const user = userEvent.setup();
		registerCore({ state: 'outdated', available: AHEAD });
		renderEngine();
		await user.click(await screen.findByRole('button', { name: 'Update' }));

		expect(await screen.findByText(/Downloading and checking the new version/)).toBeInTheDocument();
		expect(screen.getByRole('button', { name: /Updating/ })).toBeDisabled();
		expect(screen.getByRole('button', { name: 'Check for updates' })).toBeDisabled();
	});

	it('offers Restart to apply once an update is staged, naming the version', async () => {
		registerCore({ pending_activation: STAGED, available: AHEAD });
		renderEngine();

		expect(await screen.findByText(/Version stable-26\.6 is downloaded and checked/)).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Restart to apply' })).toBeEnabled();
		expect(screen.queryByRole('button', { name: 'Update' })).not.toBeInTheDocument();
	});

	it('restarts, then shows the new version with nothing left to apply', async () => {
		const user = userEvent.setup();
		registerCore({ pending_activation: STAGED, available: AHEAD });
		renderEngine();
		await user.click(await screen.findByRole('button', { name: 'Restart to apply' }));

		expect(await screen.findByText(/Restarting quiver\.core/)).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Restarting…' })).toBeDisabled();

		expect(await screen.findByText('quiver.core is up to date.', {}, { timeout: 5000 })).toBeInTheDocument();
		expect(screen.getByText('stable-26.6')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Restart to apply' })).not.toBeInTheDocument();
	});

	it('reports a failed check with the cause and retries it', async () => {
		const user = userEvent.setup();
		registerCore();
		renderEngine();
		await screen.findByText('quiver.core is up to date.');
		useMockStore.getState().setFault('arrows', 100);

		await user.click(screen.getByRole('button', { name: 'Check for updates' }));

		expect(await screen.findByText(/The update did not complete\. simulated failure/)).toBeInTheDocument();
		useMockStore.getState().resetFaults();
		await user.click(screen.getByRole('button', { name: 'Try again' }));
		expect(await screen.findByText('quiver.core is up to date.')).toBeInTheDocument();
	});

	it('retries a failed update as an update', async () => {
		const user = userEvent.setup();
		registerCore({ state: 'outdated', available: AHEAD });
		renderEngine();
		await screen.findByRole('button', { name: 'Update' });
		useMockStore.getState().setFault('runtime', 100);
		await user.click(screen.getByRole('button', { name: 'Update' }));
		await screen.findByText(/The update did not complete/);
		useMockStore.getState().resetFaults();

		await user.click(screen.getByRole('button', { name: 'Try again' }));

		expect(await screen.findByText(/Downloading and checking the new version/)).toBeInTheDocument();
	});

	it('retries a refused restart as a restart', async () => {
		const user = userEvent.setup();
		registerCore({ pending_activation: STAGED });
		renderEngine();
		await screen.findByRole('button', { name: 'Restart to apply' });
		useMockStore.getState().setFault('runtime', 100);
		await user.click(screen.getByRole('button', { name: 'Restart to apply' }));
		await screen.findByText(/The update did not complete/);
		useMockStore.getState().resetFaults();

		await user.click(screen.getByRole('button', { name: 'Try again' }));

		expect(await screen.findByText(/Restarting quiver\.core/)).toBeInTheDocument();
	});
});

describe('the Command line section of the Engine panel', () => {
	beforeEach(() => {
		installMock('normal');
		useEngineStore.setState({ view: null, rejected: [], loading: true, error: null, patchError: null });
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
	});

	it('offers to set up PATH, naming where it would be configured', async () => {
		renderEngine();
		expect(await screen.findByRole('button', { name: 'Set up PATH' })).toBeEnabled();
		expect(screen.getByText(/Configuration: \/home\/mock\/\.zshrc, \/home\/mock\/\.bashrc\./)).toBeInTheDocument();
	});

	it('shows a checking state until the status arrives', async () => {
		renderEngine();
		await screen.findByDisplayValue('49152');
		await screen.findByRole('button', { name: 'Set up PATH' });
	});

	it('sets up PATH and asks for a new terminal, without offering the button again', async () => {
		const user = userEvent.setup();
		renderEngine();
		await user.click(await screen.findByRole('button', { name: 'Set up PATH' }));

		expect(await screen.findByText(/Open a new terminal/)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Set up PATH' })).not.toBeInTheDocument();
		expect(screen.getByText('Configured')).toBeInTheDocument();
	});

	it('shows the ready state, with no button, when the directory is already on PATH', async () => {
		vi.spyOn(pathApi, 'getPathStatus').mockResolvedValue({
			binDir: '/b',
			onPath: true,
			configured: true,
			files: [],
		});
		renderEngine();
		expect(await screen.findByText('Ready')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Set up PATH' })).not.toBeInTheDocument();
		expect(screen.queryByText(/Configuration:/)).not.toBeInTheDocument();
	});

	it('shows a setup failure inline and keeps the button', async () => {
		const user = userEvent.setup();
		renderEngine();
		const button = await screen.findByRole('button', { name: 'Set up PATH' });
		useMockStore.getState().setFault('path', 100);
		await user.click(button);

		expect(await screen.findByRole('alert')).toHaveTextContent(/mock fault: path/i);
		expect(screen.getByRole('button', { name: 'Set up PATH' })).toBeEnabled();
	});

	it('offers a retry when the status cannot be loaded', async () => {
		const user = userEvent.setup();
		useMockStore.getState().setFault('path', 100);
		renderEngine();
		await screen.findByDisplayValue('49152');
		const retry = await screen.findAllByRole('button', { name: 'Try again' });

		useMockStore.getState().setFault('path', 0);
		await user.click(retry[0]);
		expect(await screen.findByRole('button', { name: 'Set up PATH' })).toBeInTheDocument();
	});

	it('hides the entry on a core without the endpoint', async () => {
		vi.spyOn(pathApi, 'getPathStatus').mockRejectedValue(new ApiError('not found', 404));
		renderEngine();
		await screen.findByDisplayValue('49152');
		await waitFor(() => expect(usePathStore.getState().unavailable).toBe(true));
		expect(screen.queryByText('Command line')).not.toBeInTheDocument();
	});
});
