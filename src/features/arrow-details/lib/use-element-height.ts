import { useCallback, useEffect, useState } from 'react';

/**
 * Tracks the ref'd element's own rendered height, in pixels -- `0` until the
 * first measurement arrives.
 *
 * Returns a callback ref for the same reason `useContainerWidthAtLeast` does:
 * the caller only mounts the measured element after its loading state clears,
 * and a plain `useRef` bound on mount would miss that node entirely.
 */
export function useElementHeight(): [number, (node: Element | null) => void] {
	const [node, setNode] = useState<Element | null>(null);
	const [height, setHeight] = useState(0);
	const ref = useCallback((next: Element | null) => setNode(next), []);

	useEffect(() => {
		if (!node) return;

		const observer = new ResizeObserver(([entry]) => {
			setHeight(entry?.contentRect.height ?? 0);
		});
		observer.observe(node);
		return () => observer.disconnect();
	}, [node]);

	return [height, ref];
}
