import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { stubLayout } from '@/__mocks__/stub-layout';

import { ConsoleLog } from './console-log';
import type { ConsoleEntry } from '../stores/console-store';

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

/** The user dragging the scrollbar to `top`. */
function scrollTo(top: number): void {
	log().scrollTop = top;
	fireEvent.scroll(log());
}

const props = { expandedId: null, onToggle: () => {}, revealKey: false } as const;

describe('the console log', () => {
	it('is a log region', () => {
		render(<ConsoleLog entries={entries(2)} expandedId={null} onToggle={() => {}} revealKey={false} />);
		expect(screen.getByRole('log')).toBeInTheDocument();
	});

	it('renders the lines it is given', () => {
		render(<ConsoleLog entries={entries(3)} expandedId={null} onToggle={() => {}} revealKey={false} />);
		expect(screen.getByText('line 1')).toBeInTheDocument();
		expect(screen.getByText('line 3')).toBeInTheDocument();
	});

	it('renders nothing for no lines', () => {
		render(<ConsoleLog entries={[]} expandedId={null} onToggle={() => {}} revealKey={false} />);
		expect(log().textContent).toBe('');
	});

	it('mounts only a window of a large buffer, not all of it', () => {
		render(<ConsoleLog entries={entries(5000)} expandedId={null} onToggle={() => {}} revealKey={false} />);
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
		render(<ConsoleLog entries={[log1]} expandedId={41} onToggle={onToggle} revealKey={false} />);
		expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'true');
		fireEvent.click(screen.getByRole('button'));
		expect(onToggle).toHaveBeenCalledWith(41);
	});

	it('starts at the newest line', () => {
		render(<ConsoleLog entries={entries(50)} {...props} />);
		expect(log().scrollTop).toBe(log().scrollHeight);
		expect(log().scrollTop).toBeGreaterThan(0);
	});

	it('follows the newest line as lines arrive', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBe(log().scrollHeight);
		rerender(<ConsoleLog entries={entries(200)} {...props} />);
		expect(log().scrollTop).toBe(log().scrollHeight);
	});

	it('renders the newest lines, not the oldest, once there are more than fit', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		rerender(<ConsoleLog entries={entries(400)} {...props} />);
		fireEvent.scroll(log());
		expect(screen.getByText('line 400')).toBeInTheDocument();
		expect(screen.queryByText('line 1')).toBeNull();
	});

	it('follows rows that turn out taller than estimated: the content grows under a view already at the bottom', () => {
		restoreLayout();
		restoreLayout = stubLayout({ width: 800, height: 400 }, 60);
		render(<ConsoleLog entries={entries(30)} {...props} />);
		// Estimated at 20px a row, measured at 60: the total height changed after the
		// first pin, and the view must have followed it.
		expect(log().scrollHeight).toBe(30 * 60);
		expect(log().scrollTop).toBe(log().scrollHeight);
	});

	it('is not un-pinned by the content growing: a scroll that did not go up is not the user leaving', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		const pinned = log().scrollTop;

		// Stale geometry after a burst of lines: the content is far taller than the
		// position, but the position did not move up.
		const inner = log().firstElementChild as HTMLElement;
		inner.style.height = '5000px';
		fireEvent.scroll(log());
		expect(log().scrollTop).toBe(pinned);

		rerender(<ConsoleLog entries={entries(80)} {...props} />);
		expect(log().scrollTop).toBe(log().scrollHeight);
	});

	it('stops following once the user scrolls up', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		scrollTo(100);

		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBe(100);
	});

	it('resumes when the user comes back to the bottom', () => {
		const { rerender } = render(<ConsoleLog entries={entries(50)} {...props} />);
		scrollTo(100);
		rerender(<ConsoleLog entries={entries(60)} {...props} />);
		expect(log().scrollTop).toBe(100);

		scrollTo(log().scrollHeight - log().clientHeight);
		rerender(<ConsoleLog entries={entries(70)} {...props} />);
		expect(log().scrollTop).toBe(log().scrollHeight);
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
		expect(log().scrollTop).toBe(log().scrollHeight);
	});

	it('does not scroll an empty log', () => {
		render(<ConsoleLog entries={[]} {...props} />);
		expect(log().scrollTop).toBe(0);
	});

	it('ignores a scroll event with no element behind it', () => {
		render(<ConsoleLog entries={entries(2)} {...props} />);
		expect(() => act(() => void fireEvent.scroll(log()))).not.toThrow();
	});
});
