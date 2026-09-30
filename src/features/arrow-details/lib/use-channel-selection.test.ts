import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import type { ArrowChannel } from '@/domain/arrow';

import { selectorFor, useChannelSelection } from './use-channel-selection';

const STABLE: ArrowChannel = { name: 'stable', kind: 'ordered', latest: 'v2', count: 2, members: ['v2', 'v1'] };
const BETA: ArrowChannel = { name: 'beta', kind: 'ordered', latest: 'v3-beta', count: 1, members: ['v3-beta'] };
const NIGHTLY: ArrowChannel = { name: 'nightly', kind: 'pointer', latest: 'nightly-latest' };

describe('selectorFor', () => {
	it('follows the channel itself when its newest version is picked', () => {
		expect(selectorFor(STABLE, 'v2')).toBe('stable');
	});

	it('pins a version that is not the channel’s newest', () => {
		expect(selectorFor(STABLE, 'v1')).toBe('v1');
	});

	it('follows a pointer channel when its own ref is kept, and pins anything typed instead', () => {
		expect(selectorFor(NIGHTLY, 'nightly-latest')).toBe('nightly');
		expect(selectorFor(NIGHTLY, 'a1b2c3d')).toBe('a1b2c3d');
	});

	it('follows the channel when no version is picked, and has nothing to say with no channel', () => {
		expect(selectorFor(STABLE, undefined)).toBe('stable');
		expect(selectorFor(undefined, 'v1')).toBeUndefined();
	});
});

describe('useChannelSelection', () => {
	it('seeds from the channel the identity follows', () => {
		const { result } = renderHook(() => useChannelSelection('beta', [STABLE, BETA]));
		expect(result.current.selectedChannel).toBe('beta');
		expect(result.current.selectedVersion).toBe('v3-beta');
		expect(result.current.selector).toBe('beta');
	});

	it('seeds a pinned identity onto the channel that lists it, at that version', () => {
		const { result } = renderHook(() => useChannelSelection('v1', [STABLE, BETA]));
		expect(result.current.selectedChannel).toBe('stable');
		expect(result.current.selectedVersion).toBe('v1');
		expect(result.current.selector).toBe('v1');
	});

	it('falls back to the first published channel for a selector no channel names', () => {
		const { result } = renderHook(() => useChannelSelection('v9.*', [BETA, STABLE]));
		expect(result.current.selectedChannel).toBe('beta');
		expect(result.current.selector).toBe('beta');
	});

	it('re-seeds once when the channel list arrives after the first render', () => {
		const { result, rerender } = renderHook(({ channels }) => useChannelSelection('stable', channels), {
			initialProps: { channels: [] as ArrowChannel[] },
		});
		expect(result.current.selector).toBeUndefined();
		rerender({ channels: [BETA, STABLE] });
		expect(result.current.selectedChannel).toBe('stable');
	});

	it('picks a channel at its own newest version', () => {
		const { result } = renderHook(() => useChannelSelection('stable', [STABLE, BETA]));
		act(() => result.current.selectChannel('beta'));
		expect(result.current.selectedVersion).toBe('v3-beta');
		expect(result.current.selector).toBe('beta');
	});

	it('picks a version inside the selected channel as a pin', () => {
		const { result } = renderHook(() => useChannelSelection('stable', [STABLE, BETA]));
		act(() => result.current.selectVersion('v1'));
		expect(result.current.selector).toBe('v1');
	});
});
