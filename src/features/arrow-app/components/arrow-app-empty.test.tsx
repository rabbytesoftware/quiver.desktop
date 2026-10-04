import { screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { ArrowAppEmpty } from './arrow-app-empty';
import { renderWithRouter } from './render-with-router';

describe('ArrowAppEmpty', () => {
	it('explains there is no interface and links to the arrow page', async () => {
		await renderWithRouter(<ArrowAppEmpty namespace="github.com/user/chat" />);

		expect(screen.getByText('This arrow is not running, so it has no interface to show.')).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Details' })).toHaveAttribute('href', '/arrow/github.com/user/chat');
	});
});
