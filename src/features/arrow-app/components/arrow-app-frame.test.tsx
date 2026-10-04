import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { ArrowAppFrame } from './arrow-app-frame';

const noop = () => {};

describe('ArrowAppFrame', () => {
	it('is a sandboxed, referrer-less iframe on the arrow origin at the surface path', () => {
		render(<ArrowAppFrame host="abc" path="/chat" windows={false} visible onRef={noop} reloadKey={0} />);
		const frame = screen.getByTitle('abc');

		expect(frame.getAttribute('src')).toBe('arrow-app://abc/chat');
		expect(frame.getAttribute('sandbox')).toBe('allow-scripts allow-same-origin');
		expect(frame.getAttribute('referrerpolicy')).toBe('no-referrer');
	});

	it('uses the Windows origin form when asked', () => {
		render(<ArrowAppFrame host="abc" path="/" windows visible onRef={noop} reloadKey={0} />);
		expect(screen.getByTitle('abc').getAttribute('src')).toBe('http://arrow-app.abc/');
	});

	it('hides without unmounting so the page keeps its state', () => {
		const { rerender } = render(
			<ArrowAppFrame host="abc" path="/" windows={false} visible onRef={noop} reloadKey={0} />
		);
		const frame = screen.getByTitle('abc');
		rerender(<ArrowAppFrame host="abc" path="/" windows={false} visible={false} onRef={noop} reloadKey={0} />);

		expect(screen.getByTitle('abc')).toBe(frame);
		expect(frame.className).toContain('hidden');
	});

	it('hands its element to onRef and clears it on unmount', () => {
		const calls: unknown[] = [];
		const { unmount } = render(
			<ArrowAppFrame host="abc" path="/" windows={false} visible onRef={(el) => calls.push(el)} reloadKey={0} />
		);
		unmount();
		expect(calls[0]).toBeInstanceOf(HTMLIFrameElement);
		expect(calls[calls.length - 1]).toBeNull();
	});

	it('a new reloadKey remounts the frame', () => {
		const { rerender } = render(
			<ArrowAppFrame host="abc" path="/" windows={false} visible onRef={noop} reloadKey={0} />
		);
		const first = screen.getByTitle('abc');
		rerender(<ArrowAppFrame host="abc" path="/" windows={false} visible onRef={noop} reloadKey={1} />);
		expect(screen.getByTitle('abc')).not.toBe(first);
	});
});
