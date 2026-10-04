import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowEntry } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';

import { arrowAppOrigin, isWindows, useSurfaceNamespaces } from './surface';

describe('arrowAppOrigin', () => {
	it('uses the custom scheme everywhere but Windows', () => {
		expect(arrowAppOrigin('abc', false)).toBe('arrow-app://abc');
	});

	it('uses the http form wry rewrites to on Windows', () => {
		expect(arrowAppOrigin('abc', true)).toBe('http://arrow-app.abc');
	});
});

describe('isWindows', () => {
	afterEach(() => vi.unstubAllGlobals());

	it('reads the user agent', () => {
		vi.stubGlobal('navigator', { userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)' });
		expect(isWindows()).toBe(true);
		vi.stubGlobal('navigator', { userAgent: 'Mozilla/5.0 (Macintosh)' });
		expect(isWindows()).toBe(false);
	});

	it('is false without a navigator', () => {
		vi.stubGlobal('navigator', undefined);
		expect(isWindows()).toBe(false);
	});
});

describe('useSurfaceNamespaces', () => {
	beforeEach(() => useArrowStore.getState().reset());

	it('lists only the arrows whose active run has a surface', () => {
		const surface = { mode: 'listen' as const, path: '/', ready: true };
		const run = { method: 'execute', variables: {}, steps: [] };
		const entry = (namespace: string, active_run: ArrowEntry['active_run']) =>
			({ namespace, active_run }) as ArrowEntry;
		useArrowStore.setState({
			arrows: new Map([
				['a/with@1', entry('a/with@1', { ...run, surface })],
				['b/without@1', entry('b/without@1', run)],
				['c/idle@1', entry('c/idle@1', null)],
			]),
		});

		const { result } = renderHook(() => useSurfaceNamespaces());
		expect(result.current).toEqual(['a/with@1']);
	});
});
