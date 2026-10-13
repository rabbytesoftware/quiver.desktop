import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowEntry, ArrowSurface } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';

import { ArrowAppHeader } from './arrow-app-header';
import { renderWithRouter } from './render-with-router';

const stop = vi.fn();

vi.mock('@/lib/core-store', async (importOriginal) => ({
	...(await importOriginal<typeof import('@/lib/core-store')>()),
	useStop: () => ({ mutate: stop }),
}));

const NS = 'github.com/user/chat';

function seedArrow(surface: ArrowSurface, extra: Partial<ArrowEntry> = {}): void {
	const entry = {
		namespace: NS,
		name: 'Chat',
		version: '1.0.0',
		icon: null,
		state: 'running',
		active_run: { method: 'execute', variables: {}, steps: [], surface },
		...extra,
	} as ArrowEntry;
	useArrowStore.setState({ arrows: new Map([[NS, entry]]) });
}

const listening = (ready: boolean): ArrowSurface => ({ mode: 'listen', path: '/', ready });

describe('ArrowAppHeader', () => {
	beforeEach(() => {
		useArrowStore.getState().reset();
		stop.mockClear();
	});

	it('shows name, version and the starting state until the surface is ready', async () => {
		seedArrow(listening(false));
		await renderWithRouter(<ArrowAppHeader namespace={NS} onDetails={() => {}} onReload={() => {}} />);

		expect(screen.getByText('Chat')).toBeInTheDocument();
		expect(screen.getByText('1.0.0')).toBeInTheDocument();
		expect(screen.getByText('Starting...')).toBeInTheDocument();
	});

	it('shows running once ready', async () => {
		seedArrow(listening(true));
		await renderWithRouter(<ArrowAppHeader namespace={NS} onDetails={() => {}} onReload={() => {}} />);

		expect(screen.getByText('Running')).toBeInTheDocument();
	});

	it('renders the arrow icon through the shared ArrowIcon, with its monogram fallback', async () => {
		seedArrow(listening(true));
		await renderWithRouter(<ArrowAppHeader namespace={NS} onDetails={() => {}} onReload={() => {}} />);

		expect(screen.getByRole('img', { name: 'Chat icon' })).toHaveTextContent('Ch');
		expect(document.querySelector('[data-slot="arrow-icon"]')).toBeInTheDocument();
	});

	it('falls back to the namespace while the arrow is unknown', async () => {
		await renderWithRouter(<ArrowAppHeader namespace={NS} onDetails={() => {}} onReload={() => {}} />);
		expect(screen.getByText(NS)).toBeInTheDocument();
	});

	it('Stop calls the stop mutation for this arrow', async () => {
		seedArrow(listening(true));
		await renderWithRouter(<ArrowAppHeader namespace={NS} onDetails={() => {}} onReload={() => {}} />);
		await userEvent.click(screen.getByRole('button', { name: 'Stop' }));

		expect(stop).toHaveBeenCalledWith({ namespace: NS });
	});

	it('Reload and Details are buttons that call their handlers', async () => {
		const onReload = vi.fn();
		const onDetails = vi.fn();
		seedArrow(listening(true));
		await renderWithRouter(<ArrowAppHeader namespace={NS} onDetails={onDetails} onReload={onReload} />);
		await userEvent.click(screen.getByRole('button', { name: 'Reload' }));
		await userEvent.click(screen.getByRole('button', { name: 'Details' }));

		expect(onReload).toHaveBeenCalled();
		expect(onDetails).toHaveBeenCalled();
		expect(screen.queryByRole('link')).not.toBeInTheDocument();
	});
});
