import { describe, expect, it, vi } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { CONSOLE_FEATURE, fetchCoreVersions, parseVersions, supportsConsole } from './versions';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

describe('parseVersions', () => {
	it('reads the wire shape', () => {
		expect(
			parseVersions({
				version: 'stable-26.5.1',
				build_id: '42',
				commit: 'abc',
				built_at: '2026-10-04T13:47:00Z',
				channel: 'stable',
				features: ['console.v1', 'x'],
				api: { supported: ['v0'], latest: 'v0' },
			})
		).toEqual({
			version: 'stable-26.5.1',
			buildId: '42',
			commit: 'abc',
			builtAt: '2026-10-04T13:47:00Z',
			channel: 'stable',
			features: ['console.v1', 'x'],
		});
	});

	it('reads an older daemon as having no features', () => {
		const old = parseVersions({ version: '1.2.3', build_id: '7', api: { supported: ['v0'] } });
		expect(old).toMatchObject({ version: '1.2.3', commit: '', builtAt: '', channel: '', features: [] });
		expect(supportsConsole(old)).toBe(false);
	});

	it('ignores values of the wrong type', () => {
		expect(parseVersions({ version: 3, features: 'console.v1' })).toMatchObject({ version: '', features: [] });
		expect(parseVersions({ features: [1, null, 'console.v1'] })?.features).toEqual(['console.v1']);
	});

	it('is null for something that is not an object', () => {
		for (const bad of [null, undefined, 'x', 3]) expect(parseVersions(bad)).toBeNull();
	});
});

describe('supportsConsole', () => {
	it('needs the feature flag', () => {
		expect(supportsConsole(parseVersions({ features: [CONSOLE_FEATURE] }))).toBe(true);
		expect(supportsConsole(parseVersions({ features: ['console.v2'] }))).toBe(false);
		expect(supportsConsole(null)).toBe(false);
	});
});

describe('fetchCoreVersions', () => {
	it('reads GET /versions', async () => {
		vi.mocked(apiFetch).mockResolvedValueOnce({ version: '1', features: ['console.v1'] });
		await expect(fetchCoreVersions()).resolves.toMatchObject({ version: '1', features: ['console.v1'] });
		expect(apiFetch).toHaveBeenCalledWith('/versions');
	});
});
