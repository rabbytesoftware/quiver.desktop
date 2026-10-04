import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';


import { parseVersions } from '@/features/console/lib/versions';
import { useBuildIndicatorStore } from '@/features/console/stores/build-indicator-store';
import { useConsoleStore } from '@/features/console/stores/console-store';
import { installBackend, resetBackend, type Backend, type BuildStamp } from '@/lib/transport/backend';

import { BuildIndicator } from './build-indicator';

// 2026-10-04T14:02:09Z
const APP_STAMP: BuildStamp = { commit: '7b4dc02' + 'a'.repeat(33), built_at: 1_791_122_529, label: null };

function stubBackend(stamp: BuildStamp | Error): void {
	installBackend({
		getBuildStamp: vi.fn(() => (stamp instanceof Error ? Promise.reject(stamp) : Promise.resolve(stamp))),
	} as unknown as Backend);
}

async function renderIndicator() {
	const view = render(<BuildIndicator />);
	// The stamp is read once on mount.
	await act(async () => {});
	return view;
}

beforeAll(() => {
	vi.stubEnv('TZ', 'UTC');
});

beforeEach(() => {
	vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', 'nightly-rolling');
	useConsoleStore.setState(useConsoleStore.getInitialState(), true);
	useBuildIndicatorStore.setState({ shown: null });
	stubBackend(APP_STAMP);
});

afterEach(() => {
	resetBackend();
	vi.unstubAllEnvs();
	vi.stubEnv('TZ', 'UTC');
});

const button = () => screen.getByRole('button', { name: 'Builds and daemon console' });

describe('the build indicator', () => {
	it('shows a placeholder for each build until it knows it', async () => {
		stubBackend(new Promise<never>(() => {}) as unknown as Error);
		installBackend({ getBuildStamp: () => new Promise(() => {}) } as unknown as Backend);
		render(<BuildIndicator />);
		expect(screen.getAllByText('—')).toHaveLength(2);
	});

	it('labels the lines core and app', async () => {
		await renderIndicator();
		expect(screen.getByText('core')).toBeInTheDocument();
		expect(screen.getByText('app')).toBeInTheDocument();
	});

	it('shows a rolling build by when it was built', async () => {
		useConsoleStore.getState().setVersions(
			parseVersions({
				version: 'nightly-latest',
				commit: '9dd0b183177a64ec71a2672d1cd7cf0c70bb4877',
				built_at: '2026-10-04T13:47:00Z',
				channel: 'nightly',
			})
		);
		await renderIndicator();

		expect(screen.getByText('nightly 10-04 13:47')).toBeInTheDocument();
		expect(screen.getByText('nightly 10-04 14:02')).toBeInTheDocument();
	});

	it('also carries the commit for the hover state, and the date-only form for a narrow rail', async () => {
		useConsoleStore.getState().setVersions(
			parseVersions({
				commit: '9dd0b183177a64ec71a2672d1cd7cf0c70bb4877',
				built_at: '2026-10-04T13:47:00Z',
				channel: 'nightly',
			})
		);
		await renderIndicator();

		expect(screen.getByText('nightly 9dd0b18')).toHaveClass('hidden', 'group-hover/indicator:inline');
		expect(screen.getByText('nightly 7b4dc02')).toHaveClass('hidden', 'group-hover/indicator:inline');
		for (const compact of screen.getAllByText('nightly 10-04')) {
			expect(compact).toHaveClass('hidden', '@max-[159px]:inline');
		}
		expect(screen.getByText('nightly 10-04 14:02')).toHaveClass('@max-[159px]:hidden');
	});

	it('hides the resting text while hovered or focused', async () => {
		await renderIndicator();
		const rest = screen.getByText('nightly 10-04 14:02').parentElement;
		expect(rest).toHaveClass('group-hover/indicator:hidden', 'group-focus-visible/indicator:hidden');
	});

	it('shows a release by its version and name, with no timestamp', async () => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', 'beta');
		stubBackend({ ...APP_STAMP, label: 'beta-26.5-2' });
		useConsoleStore.getState().setVersions(parseVersions({ version: 'stable-26.5.1', channel: 'stable' }));
		await renderIndicator();

		expect(screen.getByText('stable 26.5.1')).toBeInTheDocument();
		expect(screen.getByText('beta 26.5 #2')).toBeInTheDocument();
		expect(screen.queryByText(/10-04/)).toBeNull();
	});

	it('does not repeat a release line for a narrow rail: it is already short', async () => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', 'beta');
		stubBackend({ ...APP_STAMP, label: 'beta-26.5-2' });
		await renderIndicator();
		expect(screen.getAllByText('beta 26.5 #2')).toHaveLength(1);
	});

	it('says dev for an unstamped desktop build', async () => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', '');
		stubBackend({ commit: null, built_at: null, label: null });
		await renderIndicator();
		expect(screen.getByText('dev')).toBeInTheDocument();
	});

	it('reads an empty answer, or one missing fields, as unstamped', async () => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', '');
		installBackend({ getBuildStamp: () => Promise.resolve(undefined) } as unknown as Backend);
		const first = await renderIndicator();
		expect(screen.getByText('dev')).toBeInTheDocument();
		first.unmount();

		installBackend({ getBuildStamp: () => Promise.resolve({ commit: 'abc' }) } as unknown as Backend);
		await renderIndicator();
		expect(screen.getByText('dev')).toBeInTheDocument();
	});

	it('reads a shell that cannot answer as an unstamped build', async () => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', '');
		stubBackend(new Error('no ipc'));
		await renderIndicator();
		expect(screen.getByText('dev')).toBeInTheDocument();
	});

	it('shows a placeholder for the daemon until it has answered', async () => {
		await renderIndicator();
		expect(screen.getAllByText('—')).toHaveLength(1);
	});

	it('describes both builds in full in its tooltip', async () => {
		useConsoleStore.getState().setVersions(
			parseVersions({
				commit: '9dd0b183177a64ec71a2672d1cd7cf0c70bb4877',
				built_at: '2026-10-04T13:47:00Z',
				channel: 'nightly',
			})
		);
		await renderIndicator();

		const title = button().getAttribute('title') ?? '';
		expect(title).toContain(
			'quiver.core: nightly · 9dd0b183177a64ec71a2672d1cd7cf0c70bb4877 · 2026-10-04T13:47:00Z'
		);
		expect(title).toContain('quiver.desktop: nightly · 7b4dc02');
	});

	it('has no tooltip before it knows anything', () => {
		installBackend({ getBuildStamp: () => new Promise(() => {}) } as unknown as Backend);
		render(<BuildIndicator />);
		expect(button()).not.toHaveAttribute('title');
	});

	it('toggles the console and says whether it is open', async () => {
		await renderIndicator();
		expect(button()).toHaveAttribute('aria-expanded', 'false');
		expect(button()).toHaveAttribute('aria-controls', 'console-panel');

		await userEvent.click(button());
		expect(useConsoleStore.getState().open).toBe(true);
		expect(button()).toHaveAttribute('aria-expanded', 'true');

		await userEvent.click(button());
		expect(useConsoleStore.getState().open).toBe(false);
	});

	it('is a window handle around the button, not the button itself', async () => {
		await renderIndicator();
		expect(button().parentElement).toHaveAttribute('data-tauri-drag-region');
		expect(button()).not.toHaveAttribute('data-tauri-drag-region');
	});

	it('stops updating once unmounted', async () => {
		let resolve: (s: BuildStamp) => void = () => {};
		installBackend({ getBuildStamp: () => new Promise<BuildStamp>((r) => (resolve = r)) } as unknown as Backend);
		const view = render(<BuildIndicator />);
		view.unmount();
		await act(async () => resolve(APP_STAMP));
		expect(screen.queryByText('nightly 10-04 14:02')).toBeNull();
	});

	it('stops reporting a failure once unmounted', async () => {
		let reject: (e: Error) => void = () => {};
		installBackend({ getBuildStamp: () => new Promise<BuildStamp>((_, r) => (reject = r)) } as unknown as Backend);
		const view = render(<BuildIndicator />);
		view.unmount();
		await act(async () => reject(new Error('late')));
		expect(screen.queryByText('dev')).toBeNull();
	});
});

describe('whether the indicator is on the rail', () => {
	const gone = () => screen.queryByRole('button', { name: 'Builds and daemon console' });

	it('is on a nightly build by default', async () => {
		await renderIndicator();
		expect(button()).toBeInTheDocument();
	});

	it('is off a stable release by default, leaving only drag space where it was', async () => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', 'stable');
		const { container } = await renderIndicator();
		expect(gone()).toBeNull();
		expect(container.querySelector('[data-slot="build-indicator"]')).toBeNull();
		expect(container.querySelector('[data-tauri-drag-region]')).not.toBeNull();
	});

	it.each([['beta'], ['hotfix'], ['']])('is on for a %j build by default', async (channel) => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', channel);
		await renderIndicator();
		expect(button()).toBeInTheDocument();
	});

	it('comes back on a stable release once it is turned on', async () => {
		vi.stubEnv('VITE_QUIVER_BUILD_CHANNEL', 'stable');
		useBuildIndicatorStore.setState({ shown: true });
		await renderIndicator();
		expect(button()).toBeInTheDocument();
	});

	it('goes away on a nightly once it is turned off', async () => {
		useBuildIndicatorStore.setState({ shown: false });
		await renderIndicator();
		expect(gone()).toBeNull();
	});
});
