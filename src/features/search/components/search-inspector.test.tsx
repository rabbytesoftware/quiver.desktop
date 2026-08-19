import { useState } from 'react';

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { DiscoverySummary } from '@/domain/search';
import { createMockBackend } from '@/lib/mock';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { SearchInspector } from './search-inspector';

const JOB = { id: 'job-1', expires_at: '2026-08-18T00:00:30.000Z' };

const SUMMARY: DiscoverySummary = {
	job_id: 'job-1',
	query: 'server',
	found: 7,
	verified: 5,
	skipped: 2,
	providers: [
		{ host: 'github.com', ok: true, returned: 7, reason: null, retry_after: null },
		{ host: 'gitlab.com', ok: false, returned: 0, reason: 'rate limited', retry_after: 40 },
	],
};

const NOOP = () => {};

afterEach(() => {
	resetBackend();
});

describe('SearchInspector', () => {
	it('shows the counts and the job id from the summary', () => {
		render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />);
		expect(screen.getByText('job-1')).toBeInTheDocument();
		expect(screen.getByText('7 / 5 / 2')).toBeInTheDocument();
	});

	it('labels each pass field so its meaning does not depend on position', () => {
		render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />);

		expect(screen.getByText('Query').closest('div')).toHaveTextContent('server');
		expect(screen.getByText('Job ID').closest('div')).toHaveTextContent('job-1');
		expect(screen.getByText('Found / verified / skipped').closest('div')).toHaveTextContent('7 / 5 / 2');
		expect(screen.getByText('Expires at').closest('div')).toHaveTextContent(/2026/);
	});

	it('renders one row per provider and no more', () => {
		render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />);
		expect(document.querySelectorAll('[data-slot="inspector-host"]')).toHaveLength(2);
	});

	it('never invents a row for a host that was not asked', () => {
		render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />);
		expect(screen.queryByText(/bitbucket/i)).not.toBeInTheDocument();
	});

	it('reports a refusal with its retry, not as zero results', () => {
		render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />);
		expect(screen.getByText(/rate limited/)).toBeInTheDocument();
		expect(screen.getByText(/40/)).toBeInTheDocument();
	});

	it('says why the host group is empty mid-pass instead of showing zeroes as results', () => {
		render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={null} />);
		expect(screen.getByText(/carries no providers until the pass ends/)).toBeInTheDocument();
		expect(document.querySelectorAll('[data-slot="inspector-host"]')).toHaveLength(0);
	});

	it('renders the pass counts as zero mid-pass, never blank and never hidden', () => {
		render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={null} />);
		expect(screen.getByText('0 / 0 / 0')).toBeInTheDocument();
	});

	it('shows the running.search settings, keyed by their literal wire names', async () => {
		const mock = createMockBackend('normal');
		installBackend(mock.backend);
		try {
			render(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />);
			expect(await screen.findByText('per_provider_limit')).toBeInTheDocument();
			const source = screen.getByText('running.search');
			expect(source).toBeInTheDocument();
			// The group caption is uppercase; this literal JSON path must opt out.
			expect(source.className).toContain('normal-case');
		} finally {
			mock.dispose();
		}
	});

	it('does not fetch config while closed', () => {
		const mock = createMockBackend('normal');
		const fetchSpy = vi.spyOn(mock.backend, 'fetch');
		installBackend(mock.backend);
		try {
			render(<SearchInspector job={JOB} onOpenChange={NOOP} open={false} query="server" summary={SUMMARY} />);
			expect(fetchSpy).not.toHaveBeenCalled();
		} finally {
			mock.dispose();
		}
	});

	it('clears the previous settings on close, so reopening never shows stale config', async () => {
		const mock = createMockBackend('normal');
		installBackend(mock.backend);
		const original = mock.backend.fetch.bind(mock.backend);
		let configCalls = 0;
		const release: { current: (() => void) | null } = { current: null };

		// The first /v0/config call resolves normally; the second (the reopen)
		// is held open on purpose, so the assertion below observes exactly the
		// window between close and the new fetch landing.
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path: string, init?: RequestInit) => {
			if (path !== '/v0/config') return original(path, init);
			configCalls++;
			if (configCalls === 1) return original(path, init);
			return new Promise<Response>((resolve) => {
				release.current = () => resolve(original(path, init));
			});
		});

		try {
			const { rerender } = render(
				<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />
			);
			expect(await screen.findByText('per_provider_limit')).toBeInTheDocument();

			rerender(<SearchInspector job={JOB} onOpenChange={NOOP} open={false} query="server" summary={SUMMARY} />);
			rerender(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />);

			await waitFor(() => expect(configCalls).toBe(2));
			expect(screen.queryByText('per_provider_limit')).not.toBeInTheDocument();

			release.current?.();
			expect(await screen.findByText('per_provider_limit')).toBeInTheDocument();
		} finally {
			mock.dispose();
		}
	});

	it('fetches config once for an open session, not again on every re-render', async () => {
		const mock = createMockBackend('normal');
		const fetchSpy = vi.spyOn(mock.backend, 'fetch');
		installBackend(mock.backend);
		try {
			const { rerender } = render(
				<SearchInspector job={JOB} onOpenChange={NOOP} open query="server" summary={SUMMARY} />
			);
			await screen.findByText('per_provider_limit');
			const callsAfterOpen = fetchSpy.mock.calls.length;

			rerender(<SearchInspector job={JOB} onOpenChange={NOOP} open query="server v2" summary={SUMMARY} />);
			expect(fetchSpy.mock.calls.length).toBe(callsAfterOpen);
		} finally {
			mock.dispose();
		}
	});
});

// Wired exactly as ResultsScreen wires it: a trigger button owns `open` and
// hands SearchInspector `onOpenChange`. Base UI's Dialog is expected to trap
// and restore focus on its own -- these confirm that rather than implement it.
function InspectorWithTrigger() {
	const [open, setOpen] = useState(false);
	return (
		<>
			<button onClick={() => setOpen(true)} type="button">
				Inspect
			</button>
			<SearchInspector job={JOB} onOpenChange={setOpen} open={open} query="server" summary={SUMMARY} />
		</>
	);
}

describe('SearchInspector focus handling', () => {
	it('moves focus inside the dialog and restores it to the Inspect trigger on close', async () => {
		const user = userEvent.setup();
		render(<InspectorWithTrigger />);

		const trigger = screen.getByRole('button', { name: 'Inspect' });
		await user.click(trigger);

		const dialog = await screen.findByRole('dialog');
		await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));

		await user.keyboard('{Escape}');

		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
		expect(document.activeElement).toBe(trigger);
	});

	it('renders the focus-trap guards Base UI uses to keep Tab inside the dialog while open', async () => {
		const user = userEvent.setup();
		render(<InspectorWithTrigger />);

		await user.click(screen.getByRole('button', { name: 'Inspect' }));
		await screen.findByRole('dialog');

		expect(document.querySelectorAll('[data-base-ui-focus-guard]').length).toBeGreaterThanOrEqual(2);
	});
});
