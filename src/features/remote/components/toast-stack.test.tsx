import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { ToastStack } from './toast-stack';
import { useRemoteStore } from '../stores/remote-store';

beforeEach(() => {
	useRemoteStore.setState(useRemoteStore.getInitialState(), true);
});

describe('ToastStack', () => {
	it('renders nothing when there are no toasts', () => {
		render(<ToastStack />);
		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('renders every current toast message', () => {
		useRemoteStore.setState({
			toasts: [
				{ id: 't1', message: 'Connected to Home Lab' },
				{ id: 't2', message: 'Added Garage Server' },
			],
		});
		render(<ToastStack />);
		expect(screen.getByText('Connected to Home Lab')).toBeInTheDocument();
		expect(screen.getByText('Added Garage Server')).toBeInTheDocument();
	});
});
