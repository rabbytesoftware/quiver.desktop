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

let scrollTo: ReturnType<typeof vi.fn>;
let restoreLayout: () => void;

beforeEach(() => {
	restoreLayout = stubLayout();
	scrollTo = vi.fn();
	HTMLElement.prototype.scrollTo = scrollTo as unknown as typeof HTMLElement.prototype.scrollTo;
});

afterEach(() => {
	restoreLayout();
	// @ts-expect-error -- jsdom has none; remove what the test installed.
	delete HTMLElement.prototype.scrollTo;
});

function geometry(el: HTMLElement, g: { scrollHeight: number; clientHeight: number; scrollTop: number }) {
	Object.defineProperty(el, 'scrollHeight', { value: g.scrollHeight, configurable: true });
	Object.defineProperty(el, 'clientHeight', { value: g.clientHeight, configurable: true });
	el.scrollTop = g.scrollTop;
}

const log = () => document.querySelector('[data-slot="console-log"]') as HTMLElement;

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

	it('follows the newest line as lines arrive', () => {
		const { rerender } = render(
			<ConsoleLog entries={entries(50)} expandedId={null} onToggle={() => {}} revealKey={false} />
		);
		scrollTo.mockClear();

		rerender(<ConsoleLog entries={entries(60)} expandedId={null} onToggle={() => {}} revealKey={false} />);
		expect(scrollTo).toHaveBeenCalled();
	});

	it('stops following once the user scrolls up, and resumes when they return to the bottom', () => {
		const { rerender } = render(
			<ConsoleLog entries={entries(50)} expandedId={null} onToggle={() => {}} revealKey={false} />
		);

		geometry(log(), { scrollHeight: 1000, clientHeight: 400, scrollTop: 100 });
		fireEvent.scroll(log());
		scrollTo.mockClear();
		rerender(<ConsoleLog entries={entries(60)} expandedId={null} onToggle={() => {}} revealKey={false} />);
		expect(scrollTo).not.toHaveBeenCalled();

		geometry(log(), { scrollHeight: 1000, clientHeight: 400, scrollTop: 590 });
		fireEvent.scroll(log());
		rerender(<ConsoleLog entries={entries(70)} expandedId={null} onToggle={() => {}} revealKey={false} />);
		expect(scrollTo).toHaveBeenCalled();
	});

	it('returns to the newest line when the console is shown again', () => {
		const { rerender } = render(
			<ConsoleLog entries={entries(50)} expandedId={null} onToggle={() => {}} revealKey={false} />
		);
		geometry(log(), { scrollHeight: 1000, clientHeight: 400, scrollTop: 0 });
		fireEvent.scroll(log());
		scrollTo.mockClear();

		rerender(<ConsoleLog entries={entries(50)} expandedId={null} onToggle={() => {}} revealKey={true} />);
		expect(scrollTo).toHaveBeenCalled();
	});

	it('ignores a scroll event with no element behind it', () => {
		render(<ConsoleLog entries={entries(2)} expandedId={null} onToggle={() => {}} revealKey={false} />);
		expect(() => act(() => void fireEvent.scroll(log()))).not.toThrow();
	});
});
