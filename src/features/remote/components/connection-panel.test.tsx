import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { ConnectionPanel } from './connection-panel';
import type { ConnectionRowView } from '../lib/connection-rows';

function noop() {}

const local: ConnectionRowView = {
	id: 'local',
	name: 'Local',
	kind: 'local',
	subtitle: null,
	isLocal: true,
	isRemote: false,
	isActive: true,
	statusKind: 'ready',
	showConnect: false,
	showMenuBtn: false,
};

const homeLab: ConnectionRowView = {
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
};

const handlers = {
	openMenuId: null,
	onToggleMenu: noop,
	onCloseMenu: noop,
	onRetry: noop,
	onSwitchToLocal: noop,
	onSwitch: noop,
	onRename: noop,
	onRemove: noop,
	onAddRemote: noop,
};

describe('ConnectionPanel', () => {
	it('shows a loading skeleton instead of rows while loading', () => {
		render(<ConnectionPanel loading rows={[local, homeLab]} {...handlers} />);
		expect(screen.queryByText('Home Lab')).not.toBeInTheDocument();
	});

	it('lists every connection once loaded', () => {
		render(<ConnectionPanel loading={false} rows={[local, homeLab]} {...handlers} />);
		expect(screen.getByText('Local')).toBeInTheDocument();
		expect(screen.getByText('Home Lab')).toBeInTheDocument();
	});

	it('shows the empty state only when there are no remote rows', () => {
		render(<ConnectionPanel loading={false} rows={[local]} {...handlers} />);
		expect(screen.getByText('No remote daemons yet')).toBeInTheDocument();
	});

	it('does not show the empty state once a remote is present', () => {
		render(<ConnectionPanel loading={false} rows={[local, homeLab]} {...handlers} />);
		expect(screen.queryByText('No remote daemons yet')).not.toBeInTheDocument();
	});

	it('binds each row menu action to that row id', async () => {
		const user = userEvent.setup();
		const onSwitch = vi.fn();
		const onRename = vi.fn();
		const onRemove = vi.fn();
		render(
			<ConnectionPanel
				{...handlers}
				loading={false}
				onRemove={onRemove}
				onRename={onRename}
				onSwitch={onSwitch}
				openMenuId="home-lab"
				rows={[local, homeLab]}
			/>
		);

		await user.click(screen.getByRole('menuitem', { name: 'Switch to this connection' }));
		expect(onSwitch).toHaveBeenCalledExactlyOnceWith('home-lab');

		await user.click(screen.getByRole('menuitem', { name: 'Rename' }));
		expect(onRename).toHaveBeenCalledExactlyOnceWith('home-lab');

		await user.click(screen.getByRole('menuitem', { name: 'Remove' }));
		expect(onRemove).toHaveBeenCalledExactlyOnceWith('home-lab');
	});
});
