import type { CSSProperties } from 'react';

import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { SidebarSide } from '@/features/shell/geometry';
import { SIDEBAR_DEFAULT, SIDEBAR_MAX, SIDEBAR_MIN, useShellStore } from '@/features/shell/store';

import { ResizeHandle } from './components/resize-handle';

const setSidebarWidth = useShellStore.getState().setSidebarWidth;

function renderHandle(side: SidebarSide, width = SIDEBAR_DEFAULT) {
	useShellStore.setState({ sidebarSide: side, sidebarWidth: width });
	render(
		<div data-testid="shell" style={{ '--rail': `${width}px` } as CSSProperties}>
			<ResizeHandle />
		</div>
	);
	const shell = screen.getByTestId('shell');
	return {
		handle: screen.getByRole('separator'),
		rail: () => shell.style.getPropertyValue('--rail'),
	};
}

function dragBy(handle: HTMLElement, dx: number): void {
	fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
	fireEvent.pointerMove(handle, { pointerId: 1, clientX: dx });
	fireEvent.pointerUp(handle, { pointerId: 1, clientX: dx });
}

function committedWidth(): number {
	return useShellStore.getState().sidebarWidth;
}

beforeEach(() => {
	useShellStore.setState({ sidebarSide: 'left', sidebarWidth: SIDEBAR_DEFAULT, setSidebarWidth });
	document.documentElement.style.removeProperty('--rail');
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe('the drag direction, which flips with the side', () => {
	it('grows a LEFT-docked rail when the pointer is dragged RIGHT', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.pointerMove(handle, { pointerId: 1, clientX: 40 });
		expect(rail()).toBe('286px');

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40 });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT + 40);
	});

	it('shrinks a LEFT-docked rail when the pointer is dragged LEFT', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.pointerMove(handle, { pointerId: 1, clientX: -40 });
		expect(rail()).toBe('206px');

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: -40 });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT - 40);
	});

	it('shrinks a RIGHT-docked rail when the pointer is dragged RIGHT', () => {
		const { handle, rail } = renderHandle('right');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.pointerMove(handle, { pointerId: 1, clientX: 40 });
		expect(rail()).toBe('206px');

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40 });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT - 40);
	});

	it('grows a RIGHT-docked rail when the pointer is dragged LEFT', () => {
		const { handle, rail } = renderHandle('right');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.pointerMove(handle, { pointerId: 1, clientX: -40 });
		expect(rail()).toBe('286px');

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: -40 });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT + 40);
	});
});

describe('clamping, which applies during the drag and not only at the end', () => {
	it('stops at the floor while the pointer keeps travelling past it', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.pointerMove(handle, { pointerId: 1, clientX: -500 });
		expect(rail()).toBe(`${SIDEBAR_MIN}px`);

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: -500 });
		expect(committedWidth()).toBe(SIDEBAR_MIN);
	});

	it('stops at the ceiling while the pointer keeps travelling past it', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.pointerMove(handle, { pointerId: 1, clientX: 500 });
		expect(rail()).toBe(`${SIDEBAR_MAX}px`);

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: 500 });
		expect(committedWidth()).toBe(SIDEBAR_MAX);
	});
});

describe('what the drag writes, and when', () => {
	it('writes the store once, on release, rather than on every move', () => {
		const commit = vi.fn(setSidebarWidth);
		useShellStore.setState({ setSidebarWidth: commit });
		const { handle, rail } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		for (const clientX of [10, 20, 30, 40]) {
			fireEvent.pointerMove(handle, { pointerId: 1, clientX });
		}

		expect(commit).not.toHaveBeenCalled();
		expect(rail()).toBe('286px');
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT);

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40 });
		expect(commit).toHaveBeenCalledTimes(1);
		expect(commit).toHaveBeenCalledWith(286);
	});

	it('captures the pointer so the drag survives leaving the handle', () => {
		const capture = vi.spyOn(Element.prototype, 'setPointerCapture');
		const { handle } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 7, clientX: 0 });
		expect(capture).toHaveBeenCalledWith(7);
	});

	it('ignores pointer movement when no drag is in progress', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.pointerMove(handle, { pointerId: 1, clientX: 40 });
		expect(rail()).toBe('246px');
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT);
	});

	it('abandons the drag when the pointer capture is lost', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.lostPointerCapture(handle, { pointerId: 1 });
		fireEvent.pointerMove(handle, { pointerId: 1, clientX: 40 });
		fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40 });

		expect(rail()).toBe('246px');
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT);
	});
});

describe('the arrow keys, which flip with the side for the same reason', () => {
	it('grows a LEFT-docked rail on ArrowRight and shrinks it on ArrowLeft', () => {
		const { handle } = renderHandle('left');

		fireEvent.keyDown(handle, { key: 'ArrowRight' });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT + 8);

		fireEvent.keyDown(handle, { key: 'ArrowLeft' });
		fireEvent.keyDown(handle, { key: 'ArrowLeft' });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT - 8);
	});

	it('shrinks a RIGHT-docked rail on ArrowRight and grows it on ArrowLeft', () => {
		const { handle } = renderHandle('right');

		fireEvent.keyDown(handle, { key: 'ArrowRight' });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT - 8);

		fireEvent.keyDown(handle, { key: 'ArrowLeft' });
		fireEvent.keyDown(handle, { key: 'ArrowLeft' });
		expect(committedWidth()).toBe(SIDEBAR_DEFAULT + 8);
	});

	it('moves the rail as it nudges, not only the store', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.keyDown(handle, { key: 'ArrowRight' });
		expect(rail()).toBe('254px');
	});

	it('clamps a held key at the ceiling', () => {
		const { handle } = renderHandle('left', SIDEBAR_MAX);

		fireEvent.keyDown(handle, { key: 'ArrowRight' });
		expect(committedWidth()).toBe(SIDEBAR_MAX);
	});

	it('leaves every other key to whatever else wants it', () => {
		const { handle, rail } = renderHandle('left');

		fireEvent.keyDown(handle, { key: 'ArrowUp' });
		fireEvent.keyDown(handle, { key: 'Enter' });

		expect(committedWidth()).toBe(SIDEBAR_DEFAULT);
		expect(rail()).toBe('246px');
	});
});

describe('the separator semantics', () => {
	it('announces itself as a labelled vertical separator with a range', () => {
		const { handle } = renderHandle('left');

		expect(handle).toHaveAttribute('aria-orientation', 'vertical');
		expect(handle).toHaveAccessibleName('Resize sidebar');
		expect(handle).toHaveAttribute('aria-valuemin', String(SIDEBAR_MIN));
		expect(handle).toHaveAttribute('aria-valuemax', String(SIDEBAR_MAX));
		expect(handle).toHaveAttribute('aria-valuenow', String(SIDEBAR_DEFAULT));
	});

	it('can be reached by the keyboard at all', () => {
		const { handle } = renderHandle('left');
		expect(handle).toHaveAttribute('tabindex', '0');
	});

	it('tracks the committed width in aria-valuenow', () => {
		const { handle } = renderHandle('left');

		dragBy(handle, 40);
		expect(handle).toHaveAttribute('aria-valuenow', '286');

		fireEvent.keyDown(handle, { key: 'ArrowLeft' });
		expect(handle).toHaveAttribute('aria-valuenow', '278');
	});

	it('sits on the right edge of a left-docked rail', () => {
		expect(renderHandle('left').handle.className).toContain('-right-[3px]');
	});

	it('sits on the left edge of a right-docked rail', () => {
		expect(renderHandle('right').handle.className).toContain('-left-[3px]');
	});

	it('is seven pixels wide, which is what the offset is measured against', () => {
		expect(renderHandle('left').handle.className).toContain('w-[7px]');
	});
});

describe('the grip indicator, which has to outlive the pointer leaving the strip', () => {
	it('marks itself as dragging from the press until the release', () => {
		const { handle } = renderHandle('left');
		expect(handle).toHaveAttribute('data-dragging', 'false');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		expect(handle).toHaveAttribute('data-dragging', 'true');

		fireEvent.pointerMove(handle, { pointerId: 1, clientX: 40 });
		expect(handle).toHaveAttribute('data-dragging', 'true');

		fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40 });
		expect(handle).toHaveAttribute('data-dragging', 'false');
	});

	it('drops the mark when the pointer capture is lost', () => {
		const { handle } = renderHandle('left');

		fireEvent.pointerDown(handle, { pointerId: 1, clientX: 0 });
		fireEvent.lostPointerCapture(handle, { pointerId: 1 });
		expect(handle).toHaveAttribute('data-dragging', 'false');
	});

	it('draws the line from that mark as well as from hover', () => {
		const { handle } = renderHandle('left');

		expect(handle.className).toContain('data-[dragging=true]:after:opacity-100');
		expect(handle.className).toContain('hover:after:opacity-100');
		expect(handle.className).toContain('after:w-px');
		expect(handle.className).toContain('after:bg-muted-foreground');
	});
});

describe('finding the element that owns --rail', () => {
	it('drives an explicitly marked shell', () => {
		useShellStore.setState({ sidebarSide: 'left', sidebarWidth: SIDEBAR_DEFAULT });
		render(
			<div data-testid="shell" data-shell>
				<ResizeHandle />
			</div>
		);

		dragBy(screen.getByRole('separator'), 40);
		expect(screen.getByTestId('shell').style.getPropertyValue('--rail')).toBe('286px');
	});

	it('falls back to the document root outside a shell', () => {
		useShellStore.setState({ sidebarSide: 'left', sidebarWidth: SIDEBAR_DEFAULT });
		render(<ResizeHandle />);

		dragBy(screen.getByRole('separator'), 40);
		expect(document.documentElement.style.getPropertyValue('--rail')).toBe('286px');
	});
});
