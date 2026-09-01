import { describe, expect, it } from 'vitest';

import type { ArrowState } from '@/domain/arrow';

import { arrowTileStatus } from './arrow-tile-status';

describe('arrowTileStatus', () => {
	it.each<[ArrowState, string]>([
		['installing', 'busy'],
		['updating', 'busy'],
		['stopping', 'busy'],
		['draining', 'busy'],
		['uninstalling', 'busy'],
		['running', 'active'],
		['outdated', 'up'],
		['detached', 'problem'],
	])('surfaces a badge for %s (%s)', (state, kind) => {
		expect(arrowTileStatus({ state })?.iconKind).toBe(kind);
	});

	it.each<ArrowState>(['ready', 'absent', 'removed'])('stays silent for the steady state %s', (state) => {
		expect(arrowTileStatus({ state })).toBeNull();
	});
});
