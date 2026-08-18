import { describe, expect, it } from 'vitest';

import type { DiscoveryJobDTO, SearchResultDTO } from './search';
import { toDiscoverySummary, toSearchEntry } from './search';

const DTO: SearchResultDTO = {
	namespace: 'github.com/rabbyte/minecraft',
	name: 'Minecraft Server',
	description: 'Vanilla dedicated server.',
	tags: ['game'],
	media: { icon: 'icon.png', banner: 'banner.png' },
	versions: ['v1.21.4'],
	compatible_os: ['darwin/arm64'],
	provenance: 'installed',
	installed: true,
	known: true,
	stars: 12,
	source: 'github.com',
};

describe('toSearchEntry', () => {
	it('flattens media the way ArrowEntry does', () => {
		const entry = toSearchEntry(DTO);
		expect(entry.icon).toBe('icon.png');
		expect(entry.banner).toBe('banner.png');
		expect(entry).not.toHaveProperty('media');
	});

	it('reads an absent provenance as null rather than inventing one', () => {
		const { provenance: _drop, ...without } = DTO;
		expect(toSearchEntry(without as SearchResultDTO).provenance).toBeNull();
	});

	it('keeps installed and known apart', () => {
		const browsed = toSearchEntry({ ...DTO, installed: false, known: true, provenance: 'seen' });
		expect(browsed.installed).toBe(false);
		expect(browsed.known).toBe(true);
	});

	it('defaults absent media and source to null, and missing lists to empty', () => {
		const bare = toSearchEntry({
			namespace: 'github.com/a/b',
			name: 'B',
			description: '',
			tags: [],
			versions: [],
			compatible_os: [],
			installed: false,
			known: false,
			stars: 0,
		} as SearchResultDTO);
		expect(bare.icon).toBeNull();
		expect(bare.banner).toBeNull();
		expect(bare.source).toBeNull();
	});
});

describe('toDiscoverySummary', () => {
	it('maps counts and providers, normalising absent reason and retry to null', () => {
		const job: DiscoveryJobDTO = {
			job_id: 'job-1',
			status: 'completed',
			query: 'server',
			found: 7,
			verified: 5,
			skipped: 2,
			providers: [
				{ host: 'github.com', ok: true, returned: 7 },
				{ host: 'gitlab.com', ok: false, returned: 0, reason: 'rate limited', retry_after: 40 },
			],
		};

		const summary = toDiscoverySummary(job);
		expect(summary).toMatchObject({ job_id: 'job-1', found: 7, verified: 5, skipped: 2 });
		expect(summary.providers[0]).toEqual({
			host: 'github.com',
			ok: true,
			returned: 7,
			reason: null,
			retry_after: null,
		});
		expect(summary.providers[1]).toMatchObject({ reason: 'rate limited', retry_after: 40 });
	});
});
