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
		render(<EmptyState localError={false} phase="settled" summary={summary()} />);
		expect(screen.getByText(/Nothing matched, and every host answered/)).toBeInTheDocument();
	});

	it('never reports a refusal as no results, because that is a lie about the network', () => {
		const refused = summary({
			providers: [{ host: 'gitlab.com', ok: false, returned: 0, reason: 'rate limited', retry_after: 40 }],
		});
		render(<EmptyState localError={false} phase="settled" summary={refused} />);
		expect(screen.queryByText(/every host answered/)).not.toBeInTheDocument();
		expect(screen.getByText(/gitlab\.com/)).toBeInTheDocument();
	});

	it('distinguishes a dead daemon from an empty result', () => {
		render(<EmptyState localError phase="local" summary={null} />);
		expect(screen.getByText(/Quiver is not running/)).toBeInTheDocument();
	});

	it('renders nothing at all when idle -- no illustration, no tip', () => {
		const { container } = render(<EmptyState localError={false} phase="idle" summary={null} />);
		expect(container).toBeEmptyDOMElement();
	});

	it('renders nothing while a pass is still running, because results may still arrive', () => {
		const { container } = render(<EmptyState localError={false} phase="discovering" summary={null} />);
		expect(container).toBeEmptyDOMElement();
	});

	it('renders nothing during the local window, because no host has been asked yet', () => {
		const { container } = render(<EmptyState localError={false} phase="local" summary={null} />);
		expect(container).toBeEmptyDOMElement();
	});
});
