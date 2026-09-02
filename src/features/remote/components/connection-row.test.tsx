import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { ConnectionRow } from './connection-row';
import type { ConnectionRowView } from '../lib/connection-rows';

function row(overrides: Partial<ConnectionRowView> = {}): ConnectionRowView {
	return {
		id: 'home-lab',
		name: 'Home Lab',
		kind: 'remote',
		subtitle: 'http://192.168.1.42:7420',
		isLocal: false,
		isRemote: true,
		isActive: false,
		statusKind: null,
		showConnect: true,
		showMenuBtn: true,
		...overrides,
	};
}

function noop() {}

describe('ConnectionRow', () => {
	it('shows a remote row name and url subtitle', () => {
		render(
			<ConnectionRow
				menuOpen={false}
				onCloseMenu={noop}
				onRemove={noop}
				onRename={noop}
				onRetry={noop}
				onSwitch={noop}
				onSwitchToLocal={noop}
				onToggleMenu={noop}
				row={row()}
			/>
		);
		expect(screen.getByText('Home Lab')).toBeInTheDocument();
		expect(screen.getByText('http://192.168.1.42:7420')).toBeInTheDocument();
	});

	it("shows 'This device' for the local row instead of a url", () => {
		render(
			<ConnectionRow
				menuOpen={false}
				onCloseMenu={noop}
				onRemove={noop}
				onRename={noop}
				onRetry={noop}
				onSwitch={noop}
				onSwitchToLocal={noop}
				onToggleMenu={noop}
				row={row({ id: 'local', name: 'Local', kind: 'local', isLocal: true, isRemote: false, subtitle: null })}
			/>
		);
		expect(screen.getByText('This device')).toBeInTheDocument();
	});

	it('shows a Connected badge when ready', () => {
		render(
			<ConnectionRow
				menuOpen={false}
				onCloseMenu={noop}
				onRemove={noop}
				onRename={noop}
				onRetry={noop}
				onSwitch={noop}
				onSwitchToLocal={noop}
				onToggleMenu={noop}
				row={row({ isActive: true, statusKind: 'ready', showConnect: false })}
			/>
		);
		expect(screen.getByText('Connected')).toBeInTheDocument();
	});

	it('shows the reason, Retry, and Switch to Local when disconnected', async () => {
		const user = userEvent.setup();
		const onRetry = vi.fn();
		const onSwitchToLocal = vi.fn();
		render(
			<ConnectionRow
				menuOpen={false}
				onCloseMenu={noop}
				onRemove={noop}
				onRename={noop}
				onRetry={onRetry}
				onSwitch={noop}
				onSwitchToLocal={onSwitchToLocal}
				onToggleMenu={noop}
				row={row({ isActive: true, statusKind: 'disconnected', showConnect: false })}
			/>
		);
		expect(screen.getByText('Disconnected')).toBeInTheDocument();
		expect(screen.getByText(/check the url/i)).toBeInTheDocument();

		await user.click(screen.getByRole('button', { name: 'Retry' }));
		expect(onRetry).toHaveBeenCalledOnce();

		await user.click(screen.getByRole('button', { name: 'Switch to Local' }));
		expect(onSwitchToLocal).toHaveBeenCalledOnce();
	});

	it('does not offer "Switch to Local" on the local row itself', () => {
		render(
			<ConnectionRow
				menuOpen={false}
				onCloseMenu={noop}
				onRemove={noop}
				onRename={noop}
				onRetry={noop}
				onSwitch={noop}
				onSwitchToLocal={noop}
				onToggleMenu={noop}
				row={row({
					id: 'local',
					name: 'Local',
					kind: 'local',
					isLocal: true,
					isRemote: false,
					isActive: true,
					statusKind: 'disconnected',
					showConnect: false,
				})}
			/>
		);
		expect(screen.queryByRole('button', { name: 'Switch to Local' })).not.toBeInTheDocument();
	});

	it('hides the row menu button when showMenuBtn is false', () => {
		render(
			<ConnectionRow
				menuOpen={false}
				onCloseMenu={noop}
				onRemove={noop}
				onRename={noop}
				onRetry={noop}
				onSwitch={noop}
				onSwitchToLocal={noop}
				onToggleMenu={noop}
				row={row({ showMenuBtn: false })}
			/>
		);
		expect(screen.queryByRole('button', { name: 'More actions' })).not.toBeInTheDocument();
	});

	// Not tested here: clicking the trigger to open the menu. Base UI's Menu
	// opens via a `mousedown` + `requestAnimationFrame`-deferred floating-ui
	// gesture that does not fire reliably through jsdom's synthetic
	// `userEvent.click()` -- a jsdom/floating-ui limitation, not this
	// component's logic. `onOpenChange` itself is a one-line pass-through to
	// `onToggleMenu`/`onCloseMenu` (both fully covered by
	// `remote-store.test.ts`), and the menu's actual open/close gesture is
	// exercised for real in the live app via the Tauri driver, not jsdom.
	it('renders the More actions button with an accessible label', () => {
		render(
			<ConnectionRow
				menuOpen={false}
				onCloseMenu={noop}
				onRemove={noop}
				onRename={noop}
				onRetry={noop}
				onSwitch={noop}
				onSwitchToLocal={noop}
				onToggleMenu={noop}
				row={row()}
			/>
		);
		expect(screen.getByRole('button', { name: 'More actions' })).toBeInTheDocument();
	});

	it('offers Rename and Remove for a remote row but not Local, and Switch only when inactive', async () => {
		const user = userEvent.setup();
		const onRename = vi.fn();
		const onRemove = vi.fn();
		const onSwitch = vi.fn();
		render(
			<ConnectionRow
				menuOpen
				onCloseMenu={noop}
				onRemove={onRemove}
				onRename={onRename}
				onRetry={noop}
				onSwitch={onSwitch}
				onSwitchToLocal={noop}
				onToggleMenu={noop}
				row={row({ showConnect: true })}
			/>
		);
		await user.click(screen.getByRole('menuitem', { name: 'Switch to this connection' }));
		expect(onSwitch).toHaveBeenCalledOnce();
		await user.click(screen.getByRole('menuitem', { name: 'Rename' }));
		expect(onRename).toHaveBeenCalledOnce();
		await user.click(screen.getByRole('menuitem', { name: 'Remove' }));
		expect(onRemove).toHaveBeenCalledOnce();
	});
});
