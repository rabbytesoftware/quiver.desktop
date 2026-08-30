import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

import { CollectionArrowTile } from './collection-arrow-tile';

describe('CollectionArrowTile', () => {
	it('renders the arrow name (in both the drawn-art overlay and the caption) and its bare namespace as the caption subtitle', () => {
		render(
			<CollectionArrowTile
				arrow={{ namespace: 'github.com/rabbyte/minecraft', version: 'v1.21.4', resolved: true, name: 'Minecraft Server' }}
			/>
		);
		expect(screen.getAllByText('Minecraft Server')).toHaveLength(2);
		expect(screen.getByText('github.com/rabbyte/minecraft')).toBeInTheDocument();
	});

	it('shows the version in the info strip', () => {
		render(
			<CollectionArrowTile
				arrow={{ namespace: 'github.com/rabbyte/minecraft', version: 'v1.21.4', resolved: true, name: 'Minecraft Server' }}
			/>
		);
		expect(screen.getByText('v1.21.4')).toBeInTheDocument();
	});

	it('omits the version text when there is none', () => {
		render(<CollectionArrowTile arrow={{ namespace: 'github.com/rabbyte/minecraft', resolved: true, name: 'Minecraft Server' }} />);
		expect(screen.queryByText(/^v\d/)).not.toBeInTheDocument();
	});

	it('falls back to the namespace tail when name is somehow missing', () => {
		render(<CollectionArrowTile arrow={{ namespace: 'github.com/rabbyte/minecraft', resolved: true }} />);
		expect(screen.getAllByText('minecraft').length).toBeGreaterThan(0);
	});

	it('falls back to the whole namespace when its tail segment is empty', () => {
		render(<CollectionArrowTile arrow={{ namespace: 'github.com/rabbyte/', resolved: true }} />);
		expect(screen.getAllByText('github.com/rabbyte/').length).toBeGreaterThan(0);
	});

	it('falls back to the first path segment as owner when the namespace has two segments or fewer', () => {
		render(<CollectionArrowTile arrow={{ namespace: 'standalone-repo', resolved: true, name: 'Standalone' }} />);
		expect(screen.getAllByText('standalone-repo').length).toBeGreaterThan(0);
	});

	it('renders the ghost initial from the display name', () => {
		render(<CollectionArrowTile arrow={{ namespace: 'github.com/rabbyte/minecraft', resolved: true, name: 'Minecraft Server' }} />);
		expect(screen.getByText('M')).toBeInTheDocument();
	});

	it('renders the real data-slot markers card.css hooks its hover transform to', () => {
		const { container } = render(
			<CollectionArrowTile arrow={{ namespace: 'github.com/rabbyte/minecraft', resolved: true, name: 'Minecraft Server' }} />
		);
		expect(container.querySelector('[data-slot="arrow-card"]')).toBeInTheDocument();
		expect(container.querySelector('[data-slot="card-banner"]')).toBeInTheDocument();
	});
});
