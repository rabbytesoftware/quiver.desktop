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
				onSortChange={NOOP}
				passFailed={false}
				phase="idle"
				query=""
				sort="relevance"
				summary={null}
			/>
		);
		expect(container).toBeEmptyDOMElement();
	});

	it('does not repeat the query, which the sidebar field is already holding', () => {
		render(
			<ResultsHeader
				count={3}
				job={null}
				onInspect={NOOP}
				onSortChange={NOOP}
				passFailed={false}
				phase="local"
				query="server"
				sort="relevance"
				summary={null}
			/>
		);
		// Spec 9.2: it used to be an h2 eighteen pixels from the inverted field.
		expect(screen.queryByRole('heading')).not.toBeInTheDocument();
		expect(screen.queryByText('server')).not.toBeInTheDocument();
	});

	it('announces the count politely, once, rather than per result', () => {
		render(
			<ResultsHeader
				count={3}
				job={null}
				onInspect={NOOP}
				onSortChange={NOOP}
				passFailed={false}
				phase="local"
				query="server"
				sort="relevance"
				summary={null}
			/>
		);
		const live = screen.getByText(/3 results/);
		expect(live.closest('[aria-live]')).toHaveAttribute('aria-live', 'polite');
	});

	it('keeps a refusal legible at rest instead of in a line cards paint over', () => {
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
				onSortChange={NOOP}
				passFailed={false}
				phase="settled"
				query="server"
				sort="relevance"
				summary={summary}
			/>
		);
		// The count stands still in the header; the host, the reason and the
		// retry are one click away in the sheet (spec 11.3).
		expect(screen.getByRole('button', { name: /1 host/ })).toBeInTheDocument();
		expect(screen.getByText(/1 refused/)).toBeInTheDocument();
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
				onSortChange={NOOP}
				passFailed={false}
				phase="discovering"
				query="server"
				sort="relevance"
				summary={null}
			/>
		);
		expect(screen.queryByRole('button', { name: 'Inspect' })).not.toBeInTheDocument();

		rerender(
			<ResultsHeader
				count={0}
				job={{ id: 'job-1', expires_at: '2026-08-18T00:00:30.000Z' }}
				onInspect={onInspect}
				onSortChange={NOOP}
				passFailed={false}
				phase="discovering"
				query="server"
				sort="relevance"
				summary={null}
			/>
		);
		expect(screen.getByRole('button', { name: 'Inspect' })).toBeInTheDocument();
	});

	it('hangs the narrow row between the count and the right-hand controls', () => {
		// Spec 9.8: the row replaced a 240px rail, so where it lands in the header
		// is the point of the change, not a detail of it.
		render(
			<ResultsHeader
				count={8}
				facets={<span data-testid="facets" />}
				job={null}
				onInspect={NOOP}
				onSortChange={NOOP}
				passFailed={false}
				phase="settled"
				query="server"
				sort="relevance"
				summary={null}
			/>
		);

		const facets = screen.getByTestId('facets');
		const count = screen.getByText(/8 results/);

		expect(facets).toBeInTheDocument();
		expect(count.compareDocumentPosition(facets) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
	});

	it('states plainly that the network search did not finish', () => {
		render(
			<ResultsHeader
				count={0}
				job={null}
				onInspect={NOOP}
				onSortChange={NOOP}
				passFailed
				phase="settling"
				query="server"
				sort="relevance"
				summary={null}
			/>
		);
		expect(screen.getByText(/did not finish/)).toBeInTheDocument();
	});
});
