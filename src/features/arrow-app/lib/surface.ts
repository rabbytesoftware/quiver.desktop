import { useShallow } from 'zustand/react/shallow';

import { useArrowStore } from '@/lib/core-store';

/** The origin an arrow's page is served from, per platform form. */
export function arrowAppOrigin(host: string, windows: boolean): string {
	return windows ? `http://arrow-app.${host}` : `arrow-app://${host}`;
}

export function isWindows(): boolean {
	return typeof navigator !== 'undefined' && /windows/i.test(navigator.userAgent);
}

/** Namespaces whose arrow currently has an open surface. */
export function useSurfaceNamespaces(): string[] {
	return useArrowStore(
		useShallow((s) => [...s.arrows.values()].flatMap((a) => (a.active_run?.surface ? [a.namespace] : [])))
	);
}
