import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import type { DiscoverySummary } from '@/domain/search';

import { EmptyState } from './empty-states';

function summary(over: Partial<DiscoverySummary> = {}): DiscoverySummary {
	return {
		job_id: 'job-1',
		query: 'server',
		found: 0,
		verified: 0,
		skipped: 0,
		providers: [{ host: 'github.com', ok: true, returned: 0, reason: null, retry_after: null }],
		...over,
	};
}

describe('EmptyState', () => {
	it('says nothing matched when every host answered', () => {
		render(
			<EmptyState
				query="server"
				hasResults={false}
				localError={false}
				passFailed={false}
				phase="settled"
				summary={summary()}
			/>
		);
		expect(screen.getByText(/Nothing matched, and every host answered/)).toBeInTheDocument();
	});

	it('renders nothing when results are already on screen, whatever the phase or summary say', () => {
		const { container } = render(
			<EmptyState
				query="server"
				hasResults
				localError={false}
				passFailed={false}
				phase="settled"
				summary={summary()}
			/>
		);
		expect(container).toBeEmptyDOMElement();
	});

	it('renders nothing when results are on screen even for a dead daemon', () => {
		const { container } = render(
			<EmptyState query="server" hasResults localError passFailed={false} phase="local" summary={null} />
		);
		expect(container).toBeEmptyDOMElement();
	});

	it('never reports a refusal as no results, because that is a lie about the network', () => {
		const refused = summary({
			providers: [{ host: 'gitlab.com', ok: false, returned: 0, reason: 'rate limited', retry_after: 40 }],
		});
		render(
			<EmptyState
				query="server"
				hasResults={false}
				localError={false}
				passFailed={false}
				phase="settled"
				summary={refused}
			/>
		);
		expect(screen.queryByText(/every host answered/)).not.toBeInTheDocument();
	});

	it('says nothing at all about a refusal -- the meta line already owns it', () => {
		const refused = summary({
			providers: [{ host: 'gitlab.com', ok: false, returned: 0, reason: 'rate limited', retry_after: 40 }],
		});
		const { container } = render(
			<EmptyState
				query="server"
				hasResults
				localError={false}
				passFailed={false}
				phase="settled"
				summary={refused}
			/>
		);
		expect(container).toBeEmptyDOMElement();
	});

	it('distinguishes a dead daemon from an empty result', () => {
		render(
			<EmptyState query="server" hasResults={false} localError passFailed={false} phase="local" summary={null} />
		);
		expect(screen.getByText(/Quiver is not running/)).toBeInTheDocument();
	});

	// The resting screen carries no heading and no grid, so this line is the
	// only thing on it. The design had it; an earlier reading of the spec cut
	// it and left a blank canvas.
	it('invites the first keystroke when nothing has been asked for', () => {
		render(
			<EmptyState query="" hasResults={false} localError={false} passFailed={false} phase="idle" summary={null} />
		);
		expect(screen.getByText('Type to search.')).toBeInTheDocument();
	});

	it('treats a whitespace-only query as nothing asked for', () => {
		render(
			<EmptyState
				query="   "
				hasResults={false}
				localError={false}
				passFailed={false}
				phase="idle"
				summary={null}
			/>
		);
		expect(screen.getByText('Type to search.')).toBeInTheDocument();
	});

	// `setQuery` empties the store before Lane A answers, so a real query passes
	// through `idle` on its way in. Inviting a search there talks over one that
	// is already happening.
	it('renders nothing while a real query is still on its way in', () => {
		const { container } = render(
			<EmptyState
				query="server"
				hasResults={false}
				localError={false}
				passFailed={false}
				phase="idle"
				summary={null}
			/>
		);
		expect(container).toBeEmptyDOMElement();
	});

	it('renders nothing while a pass is still running, because results may still arrive', () => {
		const { container } = render(
			<EmptyState
				query="server"
				hasResults={false}
				localError={false}
				passFailed={false}
				phase="discovering"
				summary={null}
			/>
		);
		expect(container).toBeEmptyDOMElement();
	});

	// Spec 10.2: a timeout must never read as "every host answered" -- that
	// claims the network finished when it didn't. The header already states
	// the failure, so this follows the refusal branch's rule and says nothing.
	it('never claims every host answered when the pass timed out', () => {
		const { container } = render(
			<EmptyState
				query="server"
				hasResults={false}
				localError={false}
				passFailed
				phase="settling"
				summary={null}
			/>
		);
		expect(container).toBeEmptyDOMElement();
		expect(screen.queryByText(/every host answered/)).not.toBeInTheDocument();
	});

	it('renders nothing during the local window, because no host has been asked yet', () => {
		const { container } = render(
			<EmptyState
				query="server"
				hasResults={false}
				localError={false}
				passFailed={false}
				phase="local"
				summary={null}
			/>
		);
		expect(container).toBeEmptyDOMElement();
	});
});
