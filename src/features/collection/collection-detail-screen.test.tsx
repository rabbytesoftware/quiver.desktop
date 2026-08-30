import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, fireEvent } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, it, expect, vi } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { CollectionDetailScreen } from './collection-detail-screen';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

function renderWithClient(ui: ReactNode) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe('CollectionDetailScreen', () => {
	it('shows a loading state before the fetch resolves', () => {
		vi.mocked(apiFetch).mockReturnValue(new Promise(() => {}));
		renderWithClient(<CollectionDetailScreen namespace="github.com/rabbyte/game-servers" />);
		expect(screen.getByText(/loading/i)).toBeInTheDocument();
	});

	it('shows the collection, its resolved arrows, and the unresolved count once fetched', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/game-servers',
			name: 'Game Servers',
			followed: true,
			arrows: [
				{ namespace: 'github.com/rabbyte/minecraft@v1.21.4', resolved: true, name: 'Minecraft Server' },
				{ namespace: 'github.com/rabbyte/ark-survival@v3.1.0', resolved: false },
			],
		});

		renderWithClient(<CollectionDetailScreen namespace="github.com/rabbyte/game-servers" />);

		expect(await screen.findByText('Game Servers')).toBeInTheDocument();
		expect(screen.getAllByText('Minecraft Server').length).toBeGreaterThan(0);
		expect(screen.getByRole('button', { name: /1 unresolved/ })).toBeInTheDocument();
	});

	it('shows an error state on fetch failure', async () => {
		vi.mocked(apiFetch).mockRejectedValue(new Error('not found'));
		renderWithClient(<CollectionDetailScreen namespace="github.com/rabbyte/missing" />);
		expect(await screen.findByText(/couldn't load this collection/i)).toBeInTheDocument();
	});

	it('opens the unresolved dialog from the hero pill, with the version reattached to the route', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/game-servers',
			name: 'Game Servers',
			followed: true,
			arrows: [{ namespace: 'github.com/rabbyte/ark-survival@v3.1.0', resolved: false }],
		});

		renderWithClient(<CollectionDetailScreen namespace="github.com/rabbyte/game-servers" />);

		const pill = await screen.findByRole('button', { name: /1 unresolved/ });
		fireEvent.click(pill);
		expect(await screen.findByText('github.com/rabbyte/ark-survival@v3.1.0')).toBeInTheDocument();
	});

	it('lists an unresolved route bare when it carries no version', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/game-servers',
			name: 'Game Servers',
			followed: true,
			arrows: [{ namespace: 'github.com/rabbyte/ark-survival', resolved: false }],
		});

		renderWithClient(<CollectionDetailScreen namespace="github.com/rabbyte/game-servers" />);

		const pill = await screen.findByRole('button', { name: /1 unresolved/ });
		fireEvent.click(pill);
		expect(await screen.findByText('github.com/rabbyte/ark-survival')).toBeInTheDocument();
	});

	it('omits the unresolved pill and dialog trigger when every arrow resolved', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/homelab',
			name: 'Homelab Essentials',
			followed: false,
			arrows: [{ namespace: 'github.com/rabbyte/caddy@v2.8.4', resolved: true, name: 'Caddy' }],
		});

		renderWithClient(<CollectionDetailScreen namespace="github.com/rabbyte/homelab" />);

		expect(await screen.findByText('Homelab Essentials')).toBeInTheDocument();
		expect(screen.queryByText(/unresolved/)).not.toBeInTheDocument();
	});
});
