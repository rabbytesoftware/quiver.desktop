import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';

import { ConnectionSwitcher } from './connection-switcher';
import { useRemoteStore } from '../stores/remote-store';

const local = { id: 'local', name: 'Local', kind: 'local' as const, api_version: 'v0' };
const homeLab = { id: 'home-lab', name: 'Home Lab', kind: 'remote' as const, api_version: 'v0' };

beforeEach(() => {
	useConnectionStore.setState({ connections: [local], activeId: 'local' });
	useStatusStore.setState({ status: 'ready' });
	useRemoteStore.setState(useRemoteStore.getInitialState(), true);
});

describe('ConnectionSwitcher', () => {
	it('renders nothing when there is only one saved connection', () => {
		render(<ConnectionSwitcher />);
		expect(screen.queryByRole('button')).not.toBeInTheDocument();
	});

	it('shows the active connection name once a second connection exists', () => {
		useConnectionStore.setState({ connections: [local, homeLab], activeId: 'home-lab' });
		render(<ConnectionSwitcher />);
		expect(screen.getByText('Home Lab')).toBeInTheDocument();
	});

	it('reflects the live status via a data attribute', () => {
		useConnectionStore.setState({ connections: [local, homeLab], activeId: 'home-lab' });
		useStatusStore.setState({ status: 'disconnected' });
		render(<ConnectionSwitcher />);
		expect(screen.getByRole('button')).toHaveAttribute('data-status', 'disconnected');
	});

	it('opens the command palette when clicked', async () => {
		const user = userEvent.setup();
		useConnectionStore.setState({ connections: [local, homeLab], activeId: 'local' });
		render(<ConnectionSwitcher />);
		await user.click(screen.getByRole('button'));
		expect(useRemoteStore.getState().cmdOpen).toBe(true);
	});
});
