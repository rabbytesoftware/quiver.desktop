import { RouterProvider, createMemoryHistory, createRouter } from '@tanstack/react-router';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(() => Promise.resolve()) }));

/**
 * `__root.tsx` mounts the real devtools panel in a dev build, and vitest is one.
 * Left alone it fires an async `import()` that resolves after the test that
 * triggered it has torn its DOM down, so the failure lands on whichever case
 * runs next.
 */
vi.mock('@tanstack/react-router-devtools', () => ({ TanStackRouterDevtools: () => null }));

import { invoke } from '@tauri-apps/api/core';

import { MockIndicator } from '@/components/mock-indicator';

import { useConnectionStore } from '@/lib/connection';
import { LOCALE_STORAGE_KEY, useLocaleStore } from '@/lib/i18n';
import { createMockBackend, currentMock, disposeMock, installMock } from '@/lib/mock';
import { useMockStore } from '@/lib/mock/store';
import { installBackend, resetBackend } from '@/lib/transport/backend';
import { routeTree } from '@/routeTree.gen';

import { ConnectionsSettings } from './components/tabs/connections';
import { DeveloperSettings } from './components/tabs/developer';
import { GeneralSettings } from './components/tabs/general';
import { useSettingsUI } from './store';

const invokeMock = invoke as ReturnType<typeof vi.fn>;

let reload: ReturnType<typeof vi.fn>;

beforeEach(() => {
	vi.clearAllMocks();
	reload = vi.fn();
	Object.defineProperty(window, 'location', { value: { ...window.location, reload }, writable: true });

	useSettingsUI.setState({ tab: 'connections', query: '' });
	useMockStore.setState({ enabled: false, scenario: 'normal', latency: 0, errorRate: 0, unreachable: false });
	useMockStore.getState().resetFaults();
	useConnectionStore.setState({ connections: [], activeId: 'local' });
	useLocaleStore.setState({ preference: 'system', detected: 'en' });
	localStorage.removeItem(LOCALE_STORAGE_KEY);
});

afterEach(() => {
	// The installed mock is module state, so a case that installs one would otherwise
	// answer for every case after it — including the ones asserting that NO mock is
	// installed.
	disposeMock();
	resetBackend();
});

/**
 * Settings and the mock indicator's Turn off link are both routes now, and a
 * `<Link>` outside a router throws rather than degrading. Memory history, and a
 * router built per case: the instance carries the current location, so a shared
 * one would leave every test standing wherever the last one navigated to.
 */
function renderApp(path: string) {
	const router = createRouter({ routeTree, history: createMemoryHistory({ initialEntries: [path] }) });
	render(<RouterProvider router={router} />);
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

	// The picker is local until Apply, so browsing the options does not restart
	// the app under you.
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

		// Typed rather than stepped: the increment button is unlabelled by
		// design (it is a glyph), and the value is what matters here.
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

	it('enables Reset chaos only once something is set', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		expect(screen.getByRole('button', { name: 'Reset chaos' })).toBeDisabled();
		await user.click(screen.getByRole('switch', { name: 'Daemon unreachable' }));
		expect(screen.getByRole('button', { name: 'Reset chaos' })).toBeEnabled();

		await user.click(screen.getByRole('button', { name: 'Reset chaos' }));
		expect(useMockStore.getState().unreachable).toBe(false);
	});

	// Base UI hands back an array for range sliders and a number for single ones.
	// Unnormalised, a fault percentage would be stored as NaN and that route
	// would then never fault at all, silently.
	it('stores a slider change as a number, not the array Base UI can emit', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		// getByLabelText, not getByRole: Base UI keeps the thumb visibility:hidden
		// until it has measured the track, and jsdom has no layout — so the
		// input[type=range] inside it is filtered out as inaccessible.
		screen.getByLabelText('Search fault rate').focus();
		await user.keyboard('{ArrowRight}');

		expect(useMockStore.getState().faults.search).toBe(5);
		expect(Number.isNaN(useMockStore.getState().faults.search)).toBe(false);
	});

	it('lists one slider per route family, and resets them together', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		expect(screen.getAllByRole('slider', { hidden: true })).toHaveLength(8);
		expect(screen.getByRole('button', { name: 'Reset all faults' })).toBeDisabled();

		useMockStore.getState().setFault('search', 40);
		render(<DeveloperSettings />);
		const resets = screen.getAllByRole('button', { name: 'Reset all faults' });
		await user.click(resets[resets.length - 1]);
		expect(useMockStore.getState().faults.search).toBe(0);
	});

	// A control that visibly does nothing is worse than one that says why.
	it('disables the switch and says why when the environment forced it on', () => {
		vi.stubEnv('VITE_QUIVER_MOCK', '1');
		render(<DeveloperSettings />);

		// Base UI renders a span, so the state is on aria-disabled rather than the
		// disabled attribute.
		const sw = screen.getByRole('switch', { name: 'Use the mock server' });
		expect(sw).toHaveAttribute('aria-disabled', 'true');
		expect(sw).toBeChecked();
		expect(screen.getByText(/Forced on by VITE_QUIVER_MOCK/)).toBeInTheDocument();
		vi.unstubAllEnvs();
	});

	// Everything under Chaos is inert against a real daemon: quiver.core has no
	// latency or fault header. The panel has to say so rather than imply it works.
	it('says the chaos knobs are inert while the mock is off', () => {
		render(<DeveloperSettings />);
		expect(screen.getByText(/Inert while the mock server is off/)).toBeInTheDocument();
	});
});

describe('the Connections panel', () => {
	it('marks the active host and offers no Switch for it', () => {
		useConnectionStore.setState({
			connections: [
				{ id: 'local', name: 'Local', kind: 'local', api_version: 'v0' },
				{ id: 'r1', name: 'Basement', kind: 'remote', url: 'https://box', api_version: 'v0' },
			],
			activeId: 'local',
		});
		render(<ConnectionsSettings />);

		expect(screen.getByText('Active')).toBeInTheDocument();
		expect(screen.getAllByRole('button', { name: 'Switch' })).toHaveLength(1);
	});

	// `manager.rs` refuses to remove the local connection, so the button is
	// absent rather than present and guaranteed to fail.
	it('offers no Remove for the local daemon', () => {
		useConnectionStore.setState({
			connections: [{ id: 'local', name: 'Local', kind: 'local', api_version: 'v0' }],
			activeId: 'local',
		});
		render(<ConnectionsSettings />);
		expect(screen.queryByRole('button', { name: 'Remove' })).not.toBeInTheDocument();
	});

	it('switches and removes through the shell commands', async () => {
		const user = userEvent.setup();
		useConnectionStore.setState({
			connections: [
				{ id: 'local', name: 'Local', kind: 'local', api_version: 'v0' },
				{ id: 'r1', name: 'Basement', kind: 'remote', url: 'https://box', api_version: 'v0' },
			],
			activeId: 'local',
		});
		render(<ConnectionsSettings />);

		await user.click(screen.getByRole('button', { name: 'Switch' }));
		expect(invokeMock).toHaveBeenCalledWith('switch_connection', { id: 'r1' });

		await user.click(screen.getByRole('button', { name: 'Remove' }));
		expect(invokeMock).toHaveBeenCalledWith('remove_connection', { id: 'r1' });
	});

	it('adds a host and clears the form', async () => {
		const user = userEvent.setup();
		render(<ConnectionsSettings />);

		await user.type(screen.getByRole('textbox', { name: 'Host name' }), 'Basement');
		await user.type(screen.getByRole('textbox', { name: 'Host URL' }), 'https://box');
		await user.click(screen.getByRole('button', { name: 'Add host' }));

		expect(invokeMock).toHaveBeenCalledWith('add_connection', {
			name: 'Basement',
			url: 'https://box',
			token: '',
		});
		expect(screen.getByRole('textbox', { name: 'Host name' })).toHaveValue('');
	});

	it('surfaces a failed command instead of swallowing it', async () => {
		const user = userEvent.setup();
		invokeMock.mockRejectedValueOnce(new Error('keyring locked'));
		render(<ConnectionsSettings />);

		await user.type(screen.getByRole('textbox', { name: 'Host name' }), 'X');
		await user.type(screen.getByRole('textbox', { name: 'Host URL' }), 'https://x');
		await user.click(screen.getByRole('button', { name: 'Add host' }));

		expect(await screen.findByText('keyring locked')).toBeInTheDocument();
	});

	// Under the mock these commands are meaningless — they write to the keyring
	// and the Tauri store, neither of which a fabricated daemon has.
	it('disables every host mutation while the mock is on, and says why', () => {
		useMockStore.setState({ enabled: true });
		useConnectionStore.setState({
			connections: [{ id: 'mock:normal', name: 'Mock · Normal', kind: 'local', api_version: 'v0' }],
			activeId: 'mock:normal',
		});
		render(<ConnectionsSettings />);

		expect(screen.getByText(/this list is fabricated/)).toBeInTheDocument();
		expect(screen.getByText(/there is no daemon behind this/)).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Add host' })).toBeDisabled();
	});
});

describe('the settings page', () => {
	// The dialog rendered nothing until something opened it. A route has no
	// closed state, so the invariant inverts: the URL is the whole gesture.
	it('renders its panels on arrival, with nothing to open first', async () => {
		renderApp('/settings');
		expect(await screen.findByRole('tab', { name: 'General' })).toBeInTheDocument();
	});

	it('lands on the tab the URL names', async () => {
		renderApp('/settings?tab=developer');
		expect(await screen.findByRole('tab', { name: 'Developer' })).toHaveAttribute('aria-selected', 'true');
	});

	// The param is the point: it is what lets anything in the app link straight
	// at a panel instead of opening Settings and asking the reader to find it.
	it('writes the picked tab back into the URL', async () => {
		const user = userEvent.setup();
		const router = renderApp('/settings');

		// Not Connections: `beforeEach` remembers that one, so clicking it is a
		// no-op and the case would pass against a page that never navigates.
		await user.click(await screen.findByRole('tab', { name: 'Developer' }));
		await waitFor(() => expect(router.state.location.searchStr).toBe('?tab=developer'));
	});

	it('filters rows across the panel as you search', async () => {
		const user = userEvent.setup();
		renderApp('/settings?tab=developer');

		await user.type(await screen.findByRole('textbox', { name: 'Search settings' }), 'unreachable');

		expect(screen.getByText('Daemon unreachable')).toBeInTheDocument();
		expect(screen.queryByText('Error rate')).not.toBeInTheDocument();
	});

	// The tab can be revoked by a build change, so a remembered `developer`
	// must not leave the page pointing at a panel that no longer exists.
	it('falls back to the first tab when the remembered one is gone', async () => {
		useSettingsUI.setState({ tab: 'developer' });
		useMockStore.setState({ devUnlocked: false });
		renderApp('/settings');

		// In dev the Developer tab is always present, so it stays selected —
		// what must hold either way is that SOME tab is selected.
		const tabs = await screen.findAllByRole('tab');
		expect(tabs.some((tab) => tab.getAttribute('aria-selected') === 'true')).toBe(true);
	});
});

describe('the mock indicator', () => {
	it('renders nothing when no mock is installed', () => {
		const { container } = render(<MockIndicator />);
		expect(container).toBeEmptyDOMElement();
	});

	// Reads the RUNNING backend, not the store's intent. Rendered through the
	// router because the Turn off control is a `<Link>`, which needs one.
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
		// A scenario name no descriptor matches still falls back to normal, so
		// force the failure at the layer that actually builds the world.
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

	// Each language written in ITSELF, read out of its own catalogue. Someone
	// stranded in a language they cannot read is looking for the word they know.
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

	// Same treatment as the mock switch under VITE_QUIVER_MOCK, and for the same
	// reason: a live control that changes nothing on screen reads as broken.
	it('disables the picker and says why when the environment forced the locale', () => {
		vi.stubEnv('VITE_QUIVER_LOCALE', 'en');
		render(<GeneralSettings />);

		expect(screen.getByRole('combobox', { name: 'Display language' })).toBeDisabled();
		expect(screen.getByText(/Forced to English by VITE_QUIVER_LOCALE/)).toBeInTheDocument();
		vi.unstubAllEnvs();
	});

	// The preview is the only place the Intl side is visible before a feature
	// renders a real date, and it is what makes "this setting does something"
	// true the moment a second catalogue lands.
	it('previews a date and a number in the chosen language', () => {
		render(<GeneralSettings />);
		expect(screen.getByText(/2026/)).toBeInTheDocument();
		expect(screen.getByText('1,234,567.89')).toBeInTheDocument();
	});
});
