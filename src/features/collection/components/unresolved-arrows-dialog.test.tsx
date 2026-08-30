import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';

import { UnresolvedArrowsDialog } from './unresolved-arrows-dialog';

describe('UnresolvedArrowsDialog', () => {
	it('lists the bare routes with no invented explanation', () => {
		render(
			<UnresolvedArrowsDialog open onOpenChange={vi.fn()} routes={['github.com/rabbyte/ark-survival@v3.1.0']} />
		);
		expect(screen.getByText('github.com/rabbyte/ark-survival@v3.1.0')).toBeInTheDocument();
	});

	it('lists every route when there is more than one', () => {
		render(
			<UnresolvedArrowsDialog
				open
				onOpenChange={vi.fn()}
				routes={['github.com/rabbyte/ark-survival@v3.1.0', 'github.com/rabbyte/missing@v1.0.0']}
			/>
		);
		expect(screen.getByText('github.com/rabbyte/ark-survival@v3.1.0')).toBeInTheDocument();
		expect(screen.getByText('github.com/rabbyte/missing@v1.0.0')).toBeInTheDocument();
	});

	it('renders nothing when closed', () => {
		render(<UnresolvedArrowsDialog open={false} onOpenChange={vi.fn()} routes={['x']} />);
		expect(screen.queryByText('x')).not.toBeInTheDocument();
	});

	it('shows a title and no fabricated reason text', () => {
		render(<UnresolvedArrowsDialog open onOpenChange={vi.fn()} routes={['x']} />);
		expect(screen.getByText('Unresolved arrows')).toBeInTheDocument();
		expect(screen.queryByText(/yanked|unreachable/i)).not.toBeInTheDocument();
	});
});
