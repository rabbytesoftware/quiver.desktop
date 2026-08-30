import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

import type { CollectionArrow } from '@/domain/collection';

import { CollectionArrowGrid } from './collection-arrow-grid';

const ARROWS: CollectionArrow[] = [
	{ namespace: 'github.com/rabbyte/minecraft', resolved: true, name: 'Minecraft Server' },
	{ namespace: 'github.com/rabbyte/ark-survival', resolved: false },
];

describe('CollectionArrowGrid', () => {
	it('renders only resolved arrows', () => {
		render(<CollectionArrowGrid arrows={ARROWS} />);
		expect(screen.getAllByText('Minecraft Server').length).toBeGreaterThan(0);
		expect(screen.queryByText('ark-survival')).not.toBeInTheDocument();
	});

	it('renders nothing when every arrow is unresolved', () => {
		const { container } = render(
			<CollectionArrowGrid arrows={[{ namespace: 'github.com/rabbyte/ark-survival', resolved: false }]} />
		);
		expect(container.querySelector('.collection-member-cell')).not.toBeInTheDocument();
	});
});
