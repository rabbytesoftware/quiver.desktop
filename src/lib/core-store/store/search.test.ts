import { beforeEach, describe, expect, it } from 'vitest';

import type { SearchEntry } from '@/domain/search';

import { useSearchStore } from './search';

function entry(namespace: string, over: Partial<SearchEntry> = {}): SearchEntry {
	return {
		namespace,
		name: namespace.split('/').pop() ?? namespace,
		description: '',
		tags: [],
		icon: null,
		banner: null,
		versions: ['v1.0.0'],
		compatible_os: [],
		provenance: 'seen',
		installed: false,
		known: false,
		stars: 0,
		source: 'github.com',
		...over,
	};
}

const JOB = { id: 'job-1', expires_at: '2026-08-18T00:00:30.000Z' };

beforeEach(() => useSearchStore.getState().reset());

describe('phases', () => {
	it('starts idle with nothing in either band', () => {
		const s = useSearchStore.getState();
		expect(s.phase).toBe('idle');
		expect(s.local).toEqual([]);
		expect(s.streamed).toEqual([]);
	});

	it('moves to local when Lane A answers, even with no hits', () => {
		useSearchStore.getState().setLocal([]);
		expect(useSearchStore.getState().phase).toBe('local');
	});

	it('moves through discovering and settling to settled', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([entry('github.com/a/one', { installed: true, known: true })]);
		s().beginPass(JOB);
		expect(s().phase).toBe('discovering');
		s().endPass({ job_id: 'job-1', query: 'q', found: 0, verified: 0, skipped: 0, providers: [] });
		expect(s().phase).toBe('settling');
		s().settle([entry('github.com/a/one', { installed: true, known: true })]);
		expect(s().phase).toBe('settled');
	});
});

describe('receive — the merge', () => {
	it('appends a namespace neither band has, in arrival order', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([]);
		s().beginPass(JOB);
		s().receive(entry('github.com/a/one'));
		s().receive(entry('github.com/a/two'));
		expect(s().streamed.map((e) => e.namespace)).toEqual(['github.com/a/one', 'github.com/a/two']);
	});

	it('drops a streamed result the local band already shows', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([entry('github.com/a/one', { installed: true, known: true, provenance: 'installed' })]);
		s().beginPass(JOB);
		s().receive(entry('github.com/a/one', { known: true, provenance: 'seen' }));
		expect(s().streamed).toEqual([]);
		expect(s().local[0].provenance).toBe('installed');
	});

	it('dedups on the bare namespace, ignoring the ref', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([entry('github.com/a/one', { versions: ['v2.0.0'], installed: true, known: true })]);
		s().beginPass(JOB);
		s().receive(entry('github.com/a/one', { versions: ['v1.0.0'] }));
		expect(s().streamed).toEqual([]);
	});

	it('does not append the same streamed namespace twice', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([]);
		s().beginPass(JOB);
		s().receive(entry('github.com/a/one'));
		s().receive(entry('github.com/a/one'));
		expect(s().streamed).toHaveLength(1);
	});

	it('ignores results that arrive when no pass is running', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([]);
		s().receive(entry('github.com/a/one'));
		expect(s().streamed).toEqual([]);
	});
});

describe('settle', () => {
	it('replaces both bands so the seam cannot survive', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([entry('github.com/a/one', { installed: true, known: true })]);
		s().beginPass(JOB);
		s().receive(entry('github.com/a/two'));
		s().endPass({ job_id: 'job-1', query: 'q', found: 1, verified: 1, skipped: 0, providers: [] });
		s().settle([
			entry('github.com/a/one', { installed: true, known: true }),
			entry('github.com/a/two', { known: true }),
		]);
		expect(s().streamed).toEqual([]);
		expect(s().local).toHaveLength(2);
	});

	it('keeps both bands when the re-query fails, rather than deleting visible results', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([entry('github.com/a/one', { installed: true, known: true })]);
		s().beginPass(JOB);
		s().receive(entry('github.com/a/two'));
		s().endPass({ job_id: 'job-1', query: 'q', found: 1, verified: 1, skipped: 0, providers: [] });
		s().settleFailed();
		expect(s().phase).toBe('settling');
		expect(s().streamed).toHaveLength(1);
		expect(s().local).toHaveLength(1);
	});
});

describe('summary', () => {
	it('outlives the job, because the job 404s after its grace', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([]);
		s().beginPass(JOB);
		s().endPass({
			job_id: 'job-1',
			query: 'server',
			found: 7,
			verified: 5,
			skipped: 2,
			providers: [{ host: 'github.com', ok: true, returned: 7, reason: null, retry_after: null }],
		});
		s().settle([]);
		expect(s().summary?.verified).toBe(5);
	});
});

describe('setQuery', () => {
	it('clears both bands and the job, so results never outlive their query', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([entry('github.com/a/one')]);
		s().beginPass(JOB);
		s().receive(entry('github.com/a/two'));
		s().setQuery('other');
		expect(s().local).toEqual([]);
		expect(s().streamed).toEqual([]);
		expect(s().job).toBeNull();
		expect(s().summary).toBeNull();
	});

	it('goes back to idle on an empty query', () => {
		const s = () => useSearchStore.getState();
		s().setLocal([]);
		s().setQuery('');
		expect(s().phase).toBe('idle');
	});
});
