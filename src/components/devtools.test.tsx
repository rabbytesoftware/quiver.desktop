import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { Devtools } from './devtools';

/**
 * Stands in for the real panel. Mocked for two reasons: the real one calls
 * `useRouter`, so it needs a router context this test has no business building,
 * and a stub is the only way to tell "the devtools mounted" apart from "the
 * Suspense fallback is showing" — both are empty DOM otherwise.
 */
vi.mock('@tanstack/react-router-devtools', () => ({
	TanStackRouterDevtools: () => <div data-testid="router-devtools" />,
}));

afterEach(() => {
	vi.unstubAllEnvs();
});

describe('Devtools', () => {
	it('mounts the router devtools in a development build', async () => {
		// Vitest runs with DEV true, which is the case that has to keep working:
		// a gate that is always closed is not a gate, it is a deletion.
		render(<Devtools />);
		expect(await screen.findByTestId('router-devtools')).toBeInTheDocument();
	});

	it('renders nothing in a release build', () => {
		// `import.meta.env.DEV` is `false` in a release `vite build`, and this is
		// the branch that keeps a debug overlay out of a shipped app. It is also
		// what makes the dynamic import above unreachable, so Rollup drops the
		// devtools package from the bundle entirely rather than shipping it dark.
		vi.stubEnv('DEV', false);
		const { container } = render(<Devtools />);
		expect(container).toBeEmptyDOMElement();
		expect(screen.queryByTestId('router-devtools')).toBeNull();
	});
});
