import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { renderWithRouter } from '@/features/arrow-app/components/render-with-router';

import { OpenAppButton } from './open-app-button';

describe('OpenAppButton', () => {
	it('renders nothing without a surface', async () => {
		await renderWithRouter(<OpenAppButton namespace="a/b" surface={null} />);
		expect(screen.queryByRole('button', { name: 'Open' })).not.toBeInTheDocument();
	});

	it('is disabled until the surface is ready', async () => {
		await renderWithRouter(<OpenAppButton namespace="a/b" surface={{ mode: 'listen', path: '/', ready: false }} />);
		expect(await screen.findByRole('button', { name: 'Open' })).toBeDisabled();
	});

	it('navigates to the app route when ready', async () => {
		const { router } = await renderWithRouter(
			<OpenAppButton namespace="github.com/user/chat" surface={{ mode: 'listen', path: '/', ready: true }} />
		);
		await userEvent.click(await screen.findByRole('button', { name: 'Open' }));
		expect(router.state.location.pathname).toBe('/app/github.com/user/chat');
	});
});
