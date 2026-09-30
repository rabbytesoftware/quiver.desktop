import { describe, expect, it } from 'vitest';

import { toPathStatus } from './path';

describe('toPathStatus', () => {
	it('maps the wire fields to camelCase', () => {
		expect(
			toPathStatus({ bin_dir: '/home/u/.quiver/bin', on_path: true, configured: true, files: ['/home/u/.zshrc'] })
		).toEqual({ binDir: '/home/u/.quiver/bin', onPath: true, configured: true, files: ['/home/u/.zshrc'] });
	});

	it('reads a missing files list as empty', () => {
		expect(toPathStatus({ bin_dir: '/b', on_path: false, configured: false, files: null }).files).toEqual([]);
	});

	it('reads anything but true as false', () => {
		const status = toPathStatus({ bin_dir: '/b' } as never);
		expect(status.onPath).toBe(false);
		expect(status.configured).toBe(false);
	});
});
