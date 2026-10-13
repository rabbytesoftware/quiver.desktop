import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { CrashFallback, ErrorBoundary } from './error-boundary';

let shouldThrow = true;

function Bomb() {
	if (shouldThrow) throw new Error('boom');
	return <p>recovered</p>;
}

beforeEach(() => {
	shouldThrow = true;
	vi.spyOn(console, 'error').mockImplementation(() => undefined);
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe('ErrorBoundary', () => {
	it('renders children when nothing throws', () => {
		render(
			<ErrorBoundary>
				<p>fine</p>
			</ErrorBoundary>
		);
		expect(screen.getByText('fine')).toBeInTheDocument();
	});

	it('shows the fallback when a child throws and recovers on retry', async () => {
		render(
			<ErrorBoundary>
				<Bomb />
			</ErrorBoundary>
		);
		expect(screen.getByRole('alert')).toBeInTheDocument();
		shouldThrow = false;
		await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
		expect(screen.getByText('recovered')).toBeInTheDocument();
	});
});

describe('CrashFallback', () => {
	it('calls onRetry when the button is pressed', async () => {
		const onRetry = vi.fn();
		render(<CrashFallback onRetry={onRetry} />);
		await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
		expect(onRetry).toHaveBeenCalledOnce();
	});
});
