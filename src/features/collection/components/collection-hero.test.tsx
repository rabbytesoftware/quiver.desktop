import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, fireEvent } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, it, expect, vi } from 'vitest';

import type { CollectionDetail } from '@/domain/collection';
import { apiFetch } from '@/lib/transport/api';

import { CollectionHero } from './collection-hero';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn().mockResolvedValue(undefined) }));

const BASE: CollectionDetail = {
	namespace: 'github.com/rabbyte/game-servers',
	name: 'Game Servers',
	description: 'Everything needed to host a weekend with friends.',
	tags: [],
	followed: false,
	arrowCount: 4,
	maintainers: ['rabbyte'],
	media: {},
	arrows: [],
};

function renderWithClient(ui: ReactNode) {
	const client = new QueryClient();
	return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe('CollectionHero', () => {
	it('renders no banner element when media.banner is unset', () => {
		const { container } = renderWithClient(<CollectionHero collection={BASE} />);
		expect(container.querySelector('.collection-banner')).not.toBeInTheDocument();
	});

	it('collapses the hero row to a single column when there is no banner', () => {
		const { container } = renderWithClient(<CollectionHero collection={BASE} />);
		expect(container.querySelector('.collection-hero-row')).toHaveClass('no-banner');
	});

	it('renders the banner image when media.banner is set', () => {
		const { container } = renderWithClient(<CollectionHero collection={{ ...BASE, media: { banner: 'banner.png' } }} />);
		const banner = container.querySelector('.collection-banner') as HTMLElement | null;
		expect(banner).toBeInTheDocument();
		expect(banner?.style.backgroundImage).toContain('banner.png');
	});

	it('does not collapse the row when there is a banner', () => {
		const { container } = renderWithClient(<CollectionHero collection={{ ...BASE, media: { banner: 'banner.png' } }} />);
		expect(container.querySelector('.collection-hero-row')).not.toHaveClass('no-banner');
	});

	// CollectionHero is a pure function of its `collection` prop -- it has no
	// local followed state of its own. Clicking Follow doesn't flip the
	// label in this isolated render; that only happens once the optimistic
	// mutation updates the query cache and the parent (CollectionDetailScreen,
	// which subscribes via useCollectionDetail) re-renders with new props.
	// That round trip is covered at the screen level (Task 10); here, this
	// component's own job is just calling the right mutation with the right
	// namespace.
	it('shows Follow when not followed, and POSTs follow on click', async () => {
		renderWithClient(<CollectionHero collection={BASE} />);
		fireEvent.click(screen.getByRole('button', { name: 'Follow' }));
		await vi.waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith('/v0/collection/github.com%2Frabbyte%2Fgame-servers/follow', {
				method: 'POST',
			})
		);
	});

	it('shows Following when followed, and DELETEs follow on click', async () => {
		renderWithClient(<CollectionHero collection={{ ...BASE, followed: true }} />);
		fireEvent.click(screen.getByRole('button', { name: 'Following' }));
		await vi.waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith('/v0/collection/github.com%2Frabbyte%2Fgame-servers/follow', {
				method: 'DELETE',
			})
		);
	});

	it('shows the unresolved pill only when unresolvedCount is greater than zero', () => {
		renderWithClient(<CollectionHero collection={BASE} onUnresolvedClick={() => {}} unresolvedCount={1} />);
		expect(screen.getByRole('button', { name: /1 unresolved/ })).toBeInTheDocument();
	});

	it('omits the unresolved pill when unresolvedCount is 0 or unset', () => {
		renderWithClient(<CollectionHero collection={BASE} />);
		expect(screen.queryByText(/unresolved/)).not.toBeInTheDocument();
	});

	it('calls onUnresolvedClick when the pill is clicked', () => {
		const onUnresolvedClick = vi.fn();
		renderWithClient(<CollectionHero collection={BASE} onUnresolvedClick={onUnresolvedClick} unresolvedCount={2} />);
		fireEvent.click(screen.getByRole('button', { name: /2 unresolved/ }));
		expect(onUnresolvedClick).toHaveBeenCalledOnce();
	});

	it('omits the maintainers line when there are none', () => {
		renderWithClient(<CollectionHero collection={{ ...BASE, maintainers: [] }} />);
		expect(screen.queryByText(/maintained by/)).not.toBeInTheDocument();
	});

	it('joins multiple maintainers with a comma', () => {
		renderWithClient(<CollectionHero collection={{ ...BASE, maintainers: ['rabbyte', 'char2cs'] }} />);
		expect(screen.getByText('maintained by rabbyte, char2cs')).toBeInTheDocument();
	});
});
