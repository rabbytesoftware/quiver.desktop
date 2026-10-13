import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { InferredBadge } from './inferred-badge';

const TOOLTIP =
	"This repository doesn't publish a Quiver manifest. Quiver built one automatically from its release downloads, so install steps may need a check.";

describe('InferredBadge', () => {
	it('says the arrow was auto-made', () => {
		render(<InferredBadge />);
		expect(screen.getByText('Auto-made')).toBeInTheDocument();
	});

	it('explains itself in plain words when hovered', async () => {
		const user = userEvent.setup();
		render(<InferredBadge />);

		await user.hover(screen.getByText('Auto-made'));

		expect(await screen.findByText(TOOLTIP)).toBeInTheDocument();
	});

	it('drops the word in compact mode but keeps it as the accessible name', () => {
		const { container } = render(<InferredBadge compact />);

		expect(screen.queryByText('Auto-made')).not.toBeInTheDocument();
		expect(container.querySelector('[data-slot="inferred-badge"]')).toHaveAttribute('aria-label', 'Auto-made');
	});

	it('does not label the badge twice when the word is visible', () => {
		const { container } = render(<InferredBadge />);
		expect(container.querySelector('[data-slot="inferred-badge"]')).not.toHaveAttribute('aria-label');
	});

	it('records the confidence for debugging, and omits it when unknown', () => {
		const { container, rerender } = render(<InferredBadge confidence="medium" />);
		expect(container.querySelector('[data-slot="inferred-badge"]')).toHaveAttribute('data-confidence', 'medium');

		rerender(<InferredBadge confidence={null} />);
		expect(container.querySelector('[data-slot="inferred-badge"]')).not.toHaveAttribute('data-confidence');
	});

	it('is a plain span by default, so it never nests a button inside the link it sits in', () => {
		const { container } = render(<InferredBadge />);
		expect(container.querySelector('[data-slot="inferred-badge"]')?.tagName).toBe('SPAN');
		expect(screen.queryByRole('button')).not.toBeInTheDocument();
	});

	it('is a focusable button when it stands on its own', async () => {
		const user = userEvent.setup();
		render(<InferredBadge focusable />);
		await user.tab();
		expect(screen.getByRole('button', { name: 'Auto-made' })).toHaveFocus();
	});
});
