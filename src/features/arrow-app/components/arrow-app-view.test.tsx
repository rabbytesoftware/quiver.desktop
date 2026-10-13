import { cleanup, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowEntry, ArrowSurface } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';

import { ArrowAppView } from './arrow-app-view';
import { renderWithRouter } from './render-with-router';
import { useOpenApps } from '../store/open-apps';

vi.mock('@/lib/core-store', async (importOriginal) => ({
	...(await importOriginal<typeof import('@/lib/core-store')>()),
	useStop: () => ({ mutate: vi.fn() }),
}));

vi.mock('@/features/arrow-details/arrow-details-screen', () => ({
	ArrowDetailsScreen: ({ namespace }: { namespace: string }) => <div>details of {namespace}</div>,
}));

const NS = 'github.com/user/chat';

function seed(surface: ArrowSurface): void {
	const entry = {
		namespace: NS,
		name: 'Chat',
		version: '1.0.0',
		icon: null,
		active_run: { method: 'execute', variables: {}, steps: [], surface },
	} as unknown as ArrowEntry;
	useArrowStore.setState({ arrows: new Map([[NS, entry]]) });
}

describe('ArrowAppView', () => {
	beforeEach(() => {
		useArrowStore.getState().reset();
		useOpenApps.setState({ order: [], frames: [], visible: null, reloads: {} });
	});

	it('makes its arrow the visible app and hides it on unmount', async () => {
		seed({ mode: 'listen', path: '/', ready: true });
		await renderWithRouter(<ArrowAppView namespace={NS} onIdentityChange={() => {}} />);

		expect(useOpenApps.getState().visible).toBe(NS);
		cleanup();
		expect(useOpenApps.getState().visible).toBeNull();
	});

	it('shows a starting state under the header until the surface is ready', async () => {
		seed({ mode: 'listen', path: '/', ready: false });
		await renderWithRouter(<ArrowAppView namespace={NS} onIdentityChange={() => {}} />);

		expect(screen.getByRole('status', { busy: true })).toHaveTextContent('Starting...');
		expect(screen.getByRole('button', { name: 'Stop' })).toBeInTheDocument();
	});

	it('shows no starting state once ready', async () => {
		seed({ mode: 'listen', path: '/', ready: true });
		await renderWithRouter(<ArrowAppView namespace={NS} onIdentityChange={() => {}} />);

		expect(screen.queryByRole('status', { busy: true })).not.toBeInTheDocument();
	});

	it('Details opens the arrow details in a dialog and leaves the app visible', async () => {
		seed({ mode: 'listen', path: '/', ready: true });
		await renderWithRouter(<ArrowAppView namespace={NS} onIdentityChange={() => {}} />);
		expect(screen.queryByText(`details of ${NS}`)).not.toBeInTheDocument();

		await userEvent.click(screen.getByRole('button', { name: 'Details' }));

		expect(await screen.findByRole('dialog')).toHaveTextContent(`details of ${NS}`);
		expect(useOpenApps.getState().visible).toBe(NS);
		await userEvent.keyboard('{Escape}');
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});
});
