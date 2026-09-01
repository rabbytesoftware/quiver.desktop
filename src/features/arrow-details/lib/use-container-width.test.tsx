import { useState } from 'react';

import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { installMockResizeObserver, MockResizeObserver } from '@/__mocks__/mock-resize-observer';

import { useContainerWidthAtLeast } from './use-container-width';

let restoreResizeObserver: () => void;

beforeEach(() => {
	restoreResizeObserver = installMockResizeObserver();
});

afterEach(() => {
	restoreResizeObserver();
});

function Probe({ minWidth, mountLate }: { minWidth: number; mountLate?: boolean }) {
	const [mounted, setMounted] = useState(!mountLate);
	const [wide, ref] = useContainerWidthAtLeast(minWidth);

	if (!mounted) {
		return <button onClick={() => setMounted(true)} type="button" />;
	}
	return <div ref={ref}>{wide ? 'wide' : 'narrow'}</div>;
}

function fire(width: number) {
	const el = screen.getByText(/wide|narrow/);
	act(() => MockResizeObserver.for(el)?.fire(width));
}

describe('useContainerWidthAtLeast', () => {
	it('starts narrow before any measurement arrives', () => {
		render(<Probe minWidth={900} />);
		expect(screen.getByText('narrow')).toBeInTheDocument();
	});

	it('flips to wide once the observed width reaches the threshold', () => {
		render(<Probe minWidth={900} />);
		fire(1000);
		expect(screen.getByText('wide')).toBeInTheDocument();
	});

	it('stays narrow when the observed width is below the threshold', () => {
		render(<Probe minWidth={900} />);
		fire(500);
		expect(screen.getByText('narrow')).toBeInTheDocument();
	});

	it('flips back to narrow when the container shrinks past the threshold again', () => {
		render(<Probe minWidth={900} />);
		fire(1000);
		expect(screen.getByText('wide')).toBeInTheDocument();
		fire(500);
		expect(screen.getByText('narrow')).toBeInTheDocument();
	});

	it('treats the threshold itself as wide', () => {
		render(<Probe minWidth={900} />);
		fire(900);
		expect(screen.getByText('wide')).toBeInTheDocument();
	});

	it('disconnects the observer on unmount', () => {
		const { unmount } = render(<Probe minWidth={900} />);
		const instance = MockResizeObserver.for(screen.getByText(/wide|narrow/))!;
		const disconnect = vi.spyOn(instance, 'disconnect');
		unmount();
		expect(disconnect).toHaveBeenCalledOnce();
	});

	it('still measures the node when it only appears after the component has already mounted', () => {
		// The real caller (ArrowDetailsScreen) renders a loading state first and
		// only mounts the measured element once data arrives -- a plain `useRef`
		// bound once on mount would miss it. This is the regression that matters.
		render(<Probe minWidth={900} mountLate />);
		expect(screen.queryByText(/wide|narrow/)).not.toBeInTheDocument();

		act(() => screen.getByRole('button').click());
		expect(screen.getByText('narrow')).toBeInTheDocument();

		fire(1000);
		expect(screen.getByText('wide')).toBeInTheDocument();
	});
});
