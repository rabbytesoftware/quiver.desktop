import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(() => Promise.resolve()) }));

import { QUIVER_CORE_NAMESPACE } from '@/domain/release';
import { currentMock, disposeMock, installMock } from '@/lib/mock';
import { arrow } from '@/lib/mock/world/scenarios/kit';
import { backend, installBackend, resetBackend } from '@/lib/transport/backend';

import { CoreUpdate } from './core-update';

const KEY = `${QUIVER_CORE_NAMESPACE}@stable-1.0`;

function seedCore(extra: Parameters<typeof arrow>[0] extends infer S ? Partial<S> : never = {}) {
	currentMock()!.world.arrows.set(
		KEY,
		arrow({ namespace: QUIVER_CORE_NAMESPACE, name: 'Quiver Core', ref: 'stable-1.0', state: 'ready', ...extra })
	);
}

function renderPanel() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	render(
		<QueryClientProvider client={client}>
			<CoreUpdate />
		</QueryClientProvider>
	);
}

/** Make the next calls to `method path` answer `status` with core's error envelope, and record every call. */
function failWith(status: number, error: string, match: (method: string, path: string) => boolean) {
	const real = backend();
	const calls: string[] = [];
	installBackend({
		...real,
		fetch: (path, init) => {
			const method = init?.method ?? 'GET';
			calls.push(`${method} ${path}`);
			if (!match(method, path)) return real.fetch(path, init);
			return Promise.resolve(
				new Response(JSON.stringify({ success: false, error, data: null }), {
					status,
					headers: { 'Content-Type': 'application/json' },
				})
			);
		},
	});
	return calls;
}

beforeEach(() => installMock('normal'));
afterEach(() => {
	disposeMock();
	resetBackend();
});

describe('CoreUpdate', () => {
	it('renders nothing until quiver.core has self-registered', () => {
		renderPanel();
		expect(screen.queryByText('Installed version')).not.toBeInTheDocument();
	});

	it('shows the installed version and that nothing is ahead', async () => {
		seedCore();
		renderPanel();
		expect(await screen.findByText('stable-1.0')).toBeInTheDocument();
		expect(screen.getByText('Up to date')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Update' })).not.toBeInTheDocument();
	});

	it('shows the available version and runs the ordinary update', async () => {
		const user = userEvent.setup();
		seedCore({ state: 'outdated', available: { ref: 'stable-2.0', commit: 'abc' } });
		renderPanel();
		expect(await screen.findByText('stable-2.0')).toBeInTheDocument();

		await user.click(screen.getByRole('button', { name: 'Update' }));

		await vi.waitFor(() => expect(currentMock()!.world.arrows.get(KEY)!.state).toBe('updating'));
	});

	it('re-checks, and picks up a newer version core finds', async () => {
		const user = userEvent.setup();
		seedCore();
		renderPanel();
		await screen.findByText('stable-1.0');
		currentMock()!.world.arrows.get(KEY)!.available = { ref: 'stable-1.1', commit: 'abc' };

		await user.click(screen.getByRole('button', { name: 'Check for updates' }));

		expect(await screen.findByText('stable-1.1')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Update' })).toBeEnabled();
	});

	it('shows a refusal plainly and offers no retry when retrying cannot help', async () => {
		const user = userEvent.setup();
		seedCore({ state: 'outdated' });
		renderPanel();
		await screen.findByRole('button', { name: 'Update' });
		failWith(422, 'release has no asset for this platform', (m, p) => m === 'POST' && p.endsWith('/update'));

		await user.click(screen.getByRole('button', { name: 'Update' }));

		expect(await screen.findByText('release has no asset for this platform')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Try again' })).not.toBeInTheDocument();
	});

	it.each([429, 502])('shows a %i plainly and Try again repeats the call', async (status) => {
		const user = userEvent.setup();
		seedCore({ state: 'outdated' });
		renderPanel();
		await screen.findByRole('button', { name: 'Update' });
		const calls = failWith(status, 'try later', (m, p) => m === 'POST' && p.endsWith('/update'));

		await user.click(screen.getByRole('button', { name: 'Update' }));
		await user.click(await screen.findByRole('button', { name: 'Try again' }));

		expect(calls.filter((c) => c.startsWith('POST') && c.endsWith('/update'))).toHaveLength(2);
	});

	it('retries a failed re-check with a re-check', async () => {
		const user = userEvent.setup();
		seedCore();
		renderPanel();
		await screen.findByText('stable-1.0');
		const calls = failWith(502, 'github is down', (m) => m === 'PATCH');

		await user.click(screen.getByRole('button', { name: 'Check for updates' }));
		expect(await screen.findByText('github is down')).toBeInTheDocument();
		await user.click(screen.getByRole('button', { name: 'Try again' }));

		expect(calls.filter((c) => c.startsWith('PATCH'))).toHaveLength(2);
	});

	it('says the daemon restarts while the update runs, and blocks a second one', async () => {
		seedCore({ state: 'updating', available: { ref: 'stable-2.0', commit: 'abc' } });
		renderPanel();
		expect(await screen.findByText(/restarts to finish updating/)).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Updating…' })).toBeDisabled();
	});
});
