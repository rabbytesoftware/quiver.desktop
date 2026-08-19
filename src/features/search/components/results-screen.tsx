import { type JSX, useLayoutEffect, useRef, useState } from 'react';

import { useSearchStore } from '@/lib/core-store/store/search';

import { useSearch } from '../use-search';
import { EmptyState } from './empty-states';
import { ResultGrid } from './result-grid';
import { ResultsHeader } from './results-header';
import { SearchInspector } from './search-inspector';

interface ResultsScreenProps {
	query: string;
}

/**
 * Ordinary flow content, not a scroll container: `AppShell` already scrolls its
 * content column (app-shell.tsx:45), so a second `overflow-y-auto` here would
 * nest scrollers and strand the header inside the inner one instead of letting
 * it scroll away with the results.
 */
export function ResultsScreen({ query }: ResultsScreenProps): JSX.Element {
	useSearch(query);

	const phase = useSearchStore((s) => s.phase);
	const local = useSearchStore((s) => s.local);
	const streamed = useSearchStore((s) => s.streamed);
	const job = useSearchStore((s) => s.job);
	const summary = useSearchStore((s) => s.summary);
	const localError = useSearchStore((s) => s.localError);
	const passFailed = useSearchStore((s) => s.passFailed);

	const [inspecting, setInspecting] = useState(false);

	const gridRef = useRef<HTMLDivElement>(null);
	const positions = useRef(new Map<string, DOMRect>());

	// Measure before the store's `local`/`streamed` change moves cards, then
	// animate from the delta (spec 9.7). jsdom implements neither matchMedia
	// nor Element.animate, so both are feature-detected rather than assumed.
	useLayoutEffect(() => {
		const cards = gridRef.current?.querySelectorAll<HTMLElement>('[data-slot="arrow-card"]') ?? [];
		const reduced =
			typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

		if (!reduced) {
			for (const card of cards) {
				const before = positions.current.get(card.getAttribute('href') ?? '');
				const after = card.getBoundingClientRect();
				if (!before) continue;
				const dx = before.left - after.left;
				const dy = before.top - after.top;
				if (dx === 0 && dy === 0) continue;
				card.animate?.([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: 'none' }], {
					duration: 420,
					easing: 'cubic-bezier(.4,0,.2,1)',
				});
			}
		}

		positions.current = new Map([...cards].map((c) => [c.getAttribute('href') ?? '', c.getBoundingClientRect()]));
	}, [local, streamed]);

	return (
		<div data-testid="search-page">
			<ResultsHeader
				count={local.length + streamed.length}
				job={job}
				onInspect={() => setInspecting(true)}
				passFailed={passFailed}
				phase={phase}
				query={query}
				summary={summary}
			/>
			<div ref={gridRef}>
				<ResultGrid local={local} phase={phase} streamed={streamed} />
			</div>
			<EmptyState
				hasResults={local.length + streamed.length > 0}
				localError={localError}
				passFailed={passFailed}
				phase={phase}
				summary={summary}
			/>
			<SearchInspector job={job} onOpenChange={setInspecting} open={inspecting} query={query} summary={summary} />
		</div>
	);
}
