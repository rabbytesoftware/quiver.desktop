import { useState } from 'react';

import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { installMockResizeObserver, MockResizeObserver } from '@/__mocks__/mock-resize-observer';

import { useElementHeight } from './use-element-height';

let restoreResizeObserver: () => void;

beforeEach(() => {
	restoreResizeObserver = installMockResizeObserver();
});

afterEach(() => {
	restoreResizeObserver();
});

function Probe({ mountLate }: { mountLate?: boolean }) {
	const [mounted, setMounted] = useState(!mountLate);
	const [height, ref] = useElementHeight();

	if (!mounted) {
		return <button onClick={() => setMounted(true)} type="button" />;
	}
	return <div ref={ref}>{`height:${height}`}</div>;
}

function fire(height: number) {
	const el = screen.getByText(/^height:/);
	act(() => MockResizeObserver.for(el)?.fire(0, height));
}

describe('useElementHeight', () => {
	it('reports 0 before any measurement arrives', () => {
		render(<Probe />);
		expect(screen.getByText('height:0')).toBeInTheDocument();
	});

	it('reports the observed height, and follows it as it changes', () => {
		render(<Probe />);
		fire(1200);
		expect(screen.getByText('height:1200')).toBeInTheDocument();
		fire(640);
		expect(screen.getByText('height:640')).toBeInTheDocument();
	});

	it('reports 0 when the observer delivers no entry', () => {
		render(<Probe />);
		fire(1200);
		const instance = MockResizeObserver.for(screen.getByText(/^height:/))!;
		act(() => instance.callback([], instance as unknown as ResizeObserver));
		expect(screen.getByText('height:0')).toBeInTheDocument();
	});

	it('disconnects the observer on unmount', () => {
		const { unmount } = render(<Probe />);
		const instance = MockResizeObserver.for(screen.getByText(/^height:/))!;
		const disconnect = vi.spyOn(instance, 'disconnect');
		unmount();
		expect(disconnect).toHaveBeenCalledOnce();
	});

	it('still measures the node when it only appears after the component has already mounted', () => {
		// Same regression `useContainerWidthAtLeast` guards: ArrowDetailsScreen
		// only mounts the rail once data arrives, after a loading state.
		render(<Probe mountLate />);
		expect(screen.queryByText(/^height:/)).not.toBeInTheDocument();

		act(() => screen.getByRole('button').click());
		fire(900);
		expect(screen.getByText('height:900')).toBeInTheDocument();
	});
});
