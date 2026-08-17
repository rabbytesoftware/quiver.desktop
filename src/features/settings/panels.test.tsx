import { RouterProvider, createMemoryHistory, createRouter } from '@tanstack/react-router';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(() => Promise.resolve()) }));

vi.mock('@tanstack/react-router-devtools', () => ({ TanStackRouterDevtools: () => null }));

import { MockIndicator } from '@/components/mock-indicator';

import { useThemeStore } from '@/features/shell';
import { useShellStore } from '@/features/shell/store';
import { LOCALE_STORAGE_KEY, useLocaleStore } from '@/lib/i18n';
import { createMockBackend, currentMock, disposeMock, installMock } from '@/lib/mock';
import { useMockStore } from '@/lib/mock/store';
import { installBackend, resetBackend } from '@/lib/transport/backend';
import { routeTree } from '@/routeTree.gen';

import { DeveloperSettings } from './components/tabs/developer';
import { GeneralSettings } from './components/tabs/general';
import { useSettingsUI } from './store';

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
});

afterEach(() => {
	disposeMock();
	resetBackend();
});

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

	it('enables Reset chaos only once something is set', async () => {
		const user = userEvent.setup();
		render(<DeveloperSettings />);

		expect(screen.getByRole('button', { name: 'Reset chaos' })).toBeDisabled();
		await user.click(screen.getByRole('switch', { name: 'Daemon unreachable' }));
		expect(screen.getByRole('button', { name: 'Reset chaos' })).toBeEnabled();

		await user.click(screen.getByRole('button', { name: 'Reset chaos' }));
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

	it('disables the switch and says why when the environment forced it on', () => {
		vi.stubEnv('VITE_QUIVER_MOCK', '1');
		render(<DeveloperSettings />);

		const sw = screen.getByRole('switch', { name: 'Use the mock server' });
		expect(sw).toHaveAttribute('aria-disabled', 'true');
		expect(sw).toBeChecked();
		expect(screen.getByText(/Forced on by VITE_QUIVER_MOCK/)).toBeInTheDocument();
		vi.unstubAllEnvs();
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
});
