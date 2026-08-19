import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { DiscoverySummary } from '@/domain/search';

import { ResultsHeader } from './results-header';

const NOOP = () => {};

describe('ResultsHeader', () => {
	it('renders nothing at all when there is no query', () => {
		const { container } = render(
			<ResultsHeader
				count={0}
				job={null}
				onInspect={NOOP}
				passFailed={false}
				phase="idle"
				query=""
				summary={null}
			/>
		);
		expect(container).toBeEmptyDOMElement();
	});

	it('makes the query the heading, so the results column says what was searched', () => {
		render(
			<ResultsHeader
				count={3}
				job={null}
				onInspect={NOOP}
				passFailed={false}
				phase="local"
				query="server"
				summary={null}
			/>
		);
		expect(screen.getByRole('heading', { name: 'server' })).toBeInTheDocument();
	});

	it('announces the count politely, once, rather than per result', () => {
		render(
			<ResultsHeader
				count={3}
				job={null}
				onInspect={NOOP}
				passFailed={false}
				phase="local"
				query="server"
				summary={null}
			/>
		);
		const live = screen.getByText(/3 results/);
		expect(live.closest('[aria-live]')).toHaveAttribute('aria-live', 'polite');
	});

	it('states a refusal with its retry rather than reporting no results', () => {
		const summary: DiscoverySummary = {
			job_id: 'job-1',
			query: 'server',
			found: 0,
			verified: 0,
			skipped: 0,
			providers: [{ host: 'gitlab.com', ok: false, returned: 0, reason: 'rate limited', retry_after: 40 }],
		};
		render(
			<ResultsHeader
				count={0}
				job={null}
				onInspect={NOOP}
				passFailed={false}
				phase="settled"
				query="server"
				summary={summary}
			/>
		);
		expect(screen.getByText(/gitlab\.com refused/)).toBeInTheDocument();
		expect(screen.getByText(/Retry in 40s/)).toBeInTheDocument();
	});

	// Varies job alone -- phase and summary stay fixed -- so the assertion
	// isolates what actually gates the button (spec 9.3: "when a job exists").
	// A test that also flips phase/summary here could pass for the wrong
	// reason, which is exactly what let this button stay unreachable mid-pass.
	it('offers Inspect once a job exists, before the summary ever arrives', () => {
		const onInspect = vi.fn();
		const { rerender } = render(
			<ResultsHeader
				count={0}
				job={null}
				onInspect={onInspect}
				passFailed={false}
				phase="discovering"
				query="server"
				summary={null}
			/>
		);
		expect(screen.queryByRole('button', { name: 'Inspect' })).not.toBeInTheDocument();

		rerender(
			<ResultsHeader
				count={0}
				job={{ id: 'job-1', expires_at: '2026-08-18T00:00:30.000Z' }}
				onInspect={onInspect}
				passFailed={false}
				phase="discovering"
				query="server"
				summary={null}
			/>
		);
		expect(screen.getByRole('button', { name: 'Inspect' })).toBeInTheDocument();
	});

	it('states plainly that the network search did not finish', () => {
		render(
			<ResultsHeader
				count={0}
				job={null}
				onInspect={NOOP}
				passFailed
				phase="settling"
				query="server"
				summary={null}
			/>
		);
		expect(screen.getByText(/did not finish/)).toBeInTheDocument();
	});
});
