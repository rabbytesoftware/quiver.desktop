import { render, screen } from '@testing-library/react';
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
