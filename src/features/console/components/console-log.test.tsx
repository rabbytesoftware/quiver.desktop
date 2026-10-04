import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { stubLayout } from '@/__mocks__/stub-layout';
import type { ConsoleEntry } from '@/features/console/lib/entries';

import { ConsoleLog } from './console-log';

function entries(n: number, from = 1): ConsoleEntry[] {
	return Array.from({ length: n }, (_, i) => ({
		id: from + i,
		kind: 'raw' as const,
		text: `line ${from + i}`,
	}));
}

let restoreLayout: () => void;

beforeEach(() => {
	restoreLayout = stubLayout();
});

afterEach(() => restoreLayout());

const log = () => document.querySelector('[data-slot="console-log"]') as HTMLElement;

/** The furthest the pane can scroll: the newest line at its foot. */
const bottom = () => log().scrollHeight - log().clientHeight;

/** The user scrolling to `top`: wheeling, which is what makes the scroll theirs. */
function scrollTo(top: number): void {
	fireEvent.wheel(log());
	log().scrollTop = top;
	fireEvent.scroll(log());
}

const props = { expandedId: null, onToggle: () => {}, revealKey: false, tailKey: 0 } as const;

describe('the console log', () => {
	it('is a log region', () => {
		render(<ConsoleLog entries={entries(2)} expandedId={null} onToggle={() => {}} revealKey={false} tailKey={0} />);
		expect(screen.getByRole('log')).toBeInTheDocument();
	});

	it('renders the lines it is given', () => {
		render(<ConsoleLog entries={entries(3)} expandedId={null} onToggle={() => {}} revealKey={false} tailKey={0} />);
		expect(screen.getByText('line 1')).toBeInTheDocument();
		expect(screen.getByText('line 3')).toBeInTheDocument();
	});

	it('renders nothing for no lines', () => {
		render(<ConsoleLog entries={[]} expandedId={null} onToggle={() => {}} revealKey={false} tailKey={0} />);
		expect(log().textContent).toBe('');
	});

	it('mounts only a window of a large buffer, not all of it', () => {
		render(
			<ConsoleLog entries={entries(5000)} expandedId={null} onToggle={() => {}} revealKey={false} tailKey={0} />
		);
		const mounted = log().querySelectorAll('[data-index]').length;
		expect(mounted).toBeGreaterThan(0);
		expect(mounted).toBeLessThan(100);
	});

	it('hands a row its expanded state by id and reports a toggle with that id', () => {
		const onToggle = vi.fn();
		const log1: ConsoleEntry = {
			id: 41,
			kind: 'log',
			record: {
				seq: 1,
				iso: '2026-10-04T14:02:14Z',
				level: 'info',
				component: 'c',
				msg: 'hello',
				fields: [],
				fieldsTruncated: false,
			},
		};
		render(<ConsoleLog entries={[log1]} expandedId={41} onToggle={onToggle} revealKey={false} tailKey={0} />);
		expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'true');
		fireEvent.click(screen.getByRole('button'));
		expect(onToggle).toHaveBeenCalledWith(41);
	});

	it('starts at the newest line', () => {
		render(<ConsoleLog entries={entries(50)} {...props} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
		expect(log().scrollTop).toBeGreaterThan(0);
	});

	it('follows the newest line as lines arrive', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
		rerender(<ConsoleLog entries={entries(200)} {...props} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
	});

	it('renders the newest lines, not the oldest, once there are more than fit', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		rerender(<ConsoleLog entries={entries(400)} {...props} />);
		fireEvent.scroll(log());
		expect(screen.getByText('line 400')).toBeInTheDocument();
		expect(screen.queryByText('line 1')).toBeNull();
	});

	it('stops following once the user scrolls up', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		scrollTo(100);

		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBe(100);
	});

	it('is not stopped by a scroll the user did not make: layout and clamping scroll the pane too', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);

		// The browser moves the position (content trimmed, rows re-measured) and says so,
		// with nobody having touched the pane.
		log().scrollTop = 100;
		fireEvent.scroll(log());

		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
	});

	it('counts the wheel, touch, keys and a press on the scrollbar as the user, and nothing else', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		const moves: [string, () => void][] = [
			['wheel', () => fireEvent.wheel(log())],
			['touch', () => fireEvent.touchMove(log())],
			['key', () => fireEvent.keyDown(log(), { key: 'PageUp' })],
			['scrollbar', () => fireEvent.pointerDown(log())],
		];
		let n = 60;
		for (const [, act] of moves) {
			log().scrollTop = bottom();
			fireEvent.scroll(log());
			act();
			log().scrollTop = 100;
			fireEvent.scroll(log());
			rerender(<ConsoleLog entries={entries(n)} {...props} />);
			expect(log().scrollTop).toBe(100);
			// Back to the bottom, by the user, to follow again.
			fireEvent.wheel(log());
			log().scrollTop = bottom();
			fireEvent.scroll(log());
			n += 10;
			rerender(<ConsoleLog entries={entries(n)} {...props} />);
			n += 10;
		}
	});

	it('does not count a click on a row as the user scrolling', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		fireEvent.pointerDown(screen.getByText('line 1'));
		// The click expands the row and the content moves under the pane.
		log().scrollTop = 100;
		fireEvent.scroll(log());

		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
	});

	it("stops counting scrolls as the user's a moment after the last input", () => {
		vi.useFakeTimers();
		try {
			const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
			fireEvent.wheel(log());
			vi.advanceTimersByTime(300);
			log().scrollTop = 100;
			fireEvent.scroll(log());

			rerender(<ConsoleLog entries={entries(60)} {...props} />);
			expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
		} finally {
			vi.useRealTimers();
		}
	});

	it('resumes when the user comes back to the bottom', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		scrollTo(100);
		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBe(100);

		scrollTo(bottom());
		rerender(<ConsoleLog entries={entries(70)} {...props} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
	});

	it('stays put while the user reads, however many lines arrive', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		scrollTo(300);
		for (const n of [60, 120, 400, 900]) rerender(<ConsoleLog entries={entries(n)} {...props} />);
		expect(log().scrollTop).toBe(300);
	});

	it('returns to the newest line when the console is shown again', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		scrollTo(0);
		rerender(<ConsoleLog entries={entries(50)} {...props} revealKey={true} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
	});

	it('returns to the newest line when a command is run, however far up it was left', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		scrollTo(0);
		rerender(<ConsoleLog entries={entries(50)} {...props} tailKey={1} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());

		// ...and follows what the command prints.
		rerender(<ConsoleLog entries={entries(70)} {...props} tailKey={1} />);
		expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
	});

	it('pins again a frame later, once the browser has laid the new rows out', () => {
		const frames: FrameRequestCallback[] = [];
		const raf = vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => frames.push(cb));
		try {
			render(<ConsoleLog entries={entries(50)} {...props} />);
			log().scrollTop = 0;
			frames.forEach((cb) => cb(0));
			expect(log().scrollTop).toBeGreaterThanOrEqual(bottom());
		} finally {
			raf.mockRestore();
		}
	});

	it('cancels a pending pin when it is replaced or unmounted', () => {
		const cancel = vi.spyOn(window, 'cancelAnimationFrame');
		const { rerender, unmount } = render(<ConsoleLog entries={entries(50)} {...props} />);
		rerender(<ConsoleLog entries={entries(51)} {...props} />);
		expect(cancel).toHaveBeenCalled();
		unmount();
		cancel.mockRestore();
	});

	it('does not scroll an empty log', () => {
		render(<ConsoleLog entries={[]} {...props} />);
		expect(log().scrollTop).toBe(0);
	});

	it('is a focusable, labelled log, so the keyboard can scroll it', () => {
		render(<ConsoleLog entries={entries(2)} {...props} />);
		expect(screen.getByRole('log', { name: 'Daemon log' })).toHaveAttribute('tabindex', '0');
	});

	it('ignores a scroll event with no element behind it', () => {
		render(<ConsoleLog entries={entries(2)} {...props} />);
		expect(() => act(() => void fireEvent.scroll(log()))).not.toThrow();
	});
});
