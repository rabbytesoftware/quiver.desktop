import { describe, expect, it } from 'vitest';

import { pathState } from './path-state';

const base = { binDir: '/b', onPath: false, configured: false, files: [] };

describe('pathState', () => {
	const testCases = [
		{ name: 'on PATH and configured', status: { ...base, onPath: true, configured: true }, want: 'ready' },
		{ name: 'on PATH by other means', status: { ...base, onPath: true }, want: 'ready' },
		{ name: 'configured but not yet on PATH', status: { ...base, configured: true }, want: 'restart' },
		{ name: 'neither', status: base, want: 'setup' },
	] as const;

	for (const tc of testCases) {
		it(tc.name, () => {
			expect(pathState(tc.status)).toBe(tc.want);
		});
	}
});
