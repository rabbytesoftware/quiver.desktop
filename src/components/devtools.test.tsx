import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { Devtools } from './devtools';

vi.mock('@tanstack/react-router-devtools', () => ({
	TanStackRouterDevtools: () => <div data-testid="router-devtools" />,
}));

afterEach(() => {
	vi.unstubAllEnvs();
});

describe('Devtools', () => {
	it('mounts the router devtools in a development build', async () => {
		render(<Devtools />);
		expect(await screen.findByTestId('router-devtools')).toBeInTheDocument();
	});

	it('renders nothing in a release build', () => {
		vi.stubEnv('DEV', false);
		const { container } = render(<Devtools />);
		expect(container).toBeEmptyDOMElement();
		expect(screen.queryByTestId('router-devtools')).toBeNull();
	});
});
