import { useCallback, useEffect, useState } from 'react';

/**
 * Tracks the ref'd element's own content width rather than the viewport --
 * the sidebar is user-resizable, so a viewport media query would trigger
 * grouping while the sidebar is eating the space that grouping needs.
 *
 * Returns a callback ref rather than taking a `RefObject`: the caller here
 * (`ArrowDetailsScreen`) only renders the measured element after a loading
 * state clears, so a plain `useRef` + `[ref, minWidth]` effect would run once
 * while `ref.current` is still null and never fire again -- a callback ref
 * backed by state re-triggers the effect exactly when the node actually
 * mounts, on whichever render pass that turns out to be.
 */
export function useContainerWidthAtLeast(minWidth: number): [boolean, (node: Element | null) => void] {
	const [node, setNode] = useState<Element | null>(null);
	const [wide, setWide] = useState(false);
	const ref = useCallback((next: Element | null) => setNode(next), []);

	useEffect(() => {
		if (!node) return;

		const observer = new ResizeObserver(([entry]) => {
			setWide((entry?.contentRect.width ?? 0) >= minWidth);
		});
		observer.observe(node);
		return () => observer.disconnect();
	}, [node, minWidth]);

	return [wide, ref];
}
