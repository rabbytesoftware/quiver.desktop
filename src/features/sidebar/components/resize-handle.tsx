/*
 * A separator the user can move is the W3C APG's window-splitter pattern:
 * `role="separator"` on a focusable element, carrying the value it controls.
 * jsx-a11y cannot express that. `aria-query` files `separator` under
 * `structure`, which is only true of the decorative kind, so the plugin reads a
 * focusable one as a static div wearing a container role and reports both the
 * `tabIndex` and the `onKeyDown`.
 *
 * Disabled here rather than in `eslint.config.js` because the config cannot say
 * this any more precisely: `no-noninteractive-tabindex` takes a role allowlist,
 * but `no-noninteractive-element-interactions` takes only a per-TAG one — so
 * fixing it centrally means allowing `onKeyDown` on every `div` in the app. A
 * file with one component in it is the narrowest scope available.
 *
 * The alternative that lints clean is dropping `tabIndex`, which means the rail
 * can only be resized with a pointer.
 */
/* eslint-disable jsx-a11y/no-noninteractive-element-interactions, jsx-a11y/no-noninteractive-tabindex */

import { useRef, useState, type JSX, type KeyboardEvent, type PointerEvent } from 'react';

import { normaliseWidth, SIDEBAR_MAX, SIDEBAR_MIN, useShellStore } from '@/features/shell/store';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

/**
 * Arrow-key step, in CSS px. Small enough that a held key reads as a resize
 * rather than a jump, large enough that crossing the 160 wide span is twenty
 * presses instead of a hundred and sixty.
 */
const NUDGE = 8;

interface Drag {
	/** Where the pointer went down, in client px. */
	originX: number;
	/** The rail's width at that moment — the drag is relative to it, not to whatever the store says now. */
	originWidth: number;
	/** Resolved once, on pointerdown: `closest` on every move would be a DOM walk per frame. */
	shell: HTMLElement;
}

/**
 * The element `--rail` has to be written to.
 *
 * `AppShell` drives `--rail` from an inline style, and an inline declaration
 * beats the `:root` rule in index.css — so writing the live width to the
 * document root would leave the grid frozen until the store commits on
 * pointer-up, and the rail would snap to its final width instead of following
 * the cursor. The selector finds that element whether it marks itself with
 * `data-shell` or merely carries the property inline. The root is the fallback
 * for a handle mounted outside a shell, where nothing else declares `--rail`
 * either.
 */
function shellOf(handle: HTMLElement): HTMLElement {
	return handle.closest<HTMLElement>('[data-shell], [style*="--rail"]') ?? document.documentElement;
}

function paint(shell: HTMLElement, width: number): void {
	shell.style.setProperty('--rail', `${width}px`);
}

/**
 * The strip itself, and the hairline that is the only thing announcing it.
 *
 * Seven pixels straddling the rail's edge rather than four sitting inside it:
 * the offset is what puts half the target on the content side, where the cursor
 * already is when it arrives at the edge. `-3px` and `7px` are design.pen's, and
 * they are a pair — the offset is what centres the strip on the edge, so moving
 * one without the other slides the whole target off it.
 *
 * `after:left-[3px]` is NOT mirrored with the side, and looks like the one place
 * the flip was forgotten. It is not: 3 + 1 + 3 is the whole seven, so the line
 * sits at the strip's centre — which is the rail's own edge, on either side. A
 * `right-[3px]` twin would resolve to the same pixel. The mirror belongs on the
 * strip's offset, which is where the side ternary already puts it.
 *
 * The line is revealed by `data-dragging` as well as by `:hover` because the
 * pointer is captured on the way down and leaves the seven-pixel strip within a
 * frame or two of the drag starting. On `:hover` alone it would blink out at the
 * exact moment the user is watching the edge move, and come back on release.
 * That is also why the strip no longer paints `--sidebar-accent` on hover: at
 * seven pixels a fill bleeds three of them over the content column, and this
 * hairline is the affordance design.pen actually specifies.
 */
const GRIP = [
	// Absolute, so the strip overlays the rail's edge instead of taking a row of
	// its own; `touch-none` because a touch drag is otherwise cancelled by the
	// scroll it starts.
	'absolute inset-y-0 z-10 w-[7px] cursor-col-resize touch-none focus-visible:outline-2',
	"after:absolute after:inset-y-0 after:left-[3px] after:w-px after:content-['']",
	'after:bg-muted-foreground after:opacity-0',
	'hover:after:opacity-100 data-[dragging=true]:after:opacity-100',
].join(' ');

/**
 * The rail's drag handle: a seven-pixel strip straddling the rail's
 * content-facing edge.
 */
export function ResizeHandle(): JSX.Element {
	const { t } = useTranslation();
	const side = useShellStore((s) => s.sidebarSide);
	const width = useShellStore((s) => s.sidebarWidth);
	const commit = useShellStore((s) => s.setSidebarWidth);

	// The drag lives in a ref, not in state. A `setState` per `pointermove` is
	// sixty renders a second of a rail whose only changing pixel is a width the
	// DOM node already has.
	const drag = useRef<Drag | null>(null);

	// Whether a drag is in progress is the one part of it CSS has to see, so it
	// is mirrored in state: twice per drag, on the way down and on the way up,
	// not once per move. It cannot be read off the ref — mutating a ref renders
	// nothing, so the attribute would still say what it said last render.
	const [dragging, setDragging] = useState(false);

	/**
	 * Spec §5.12, and the one line in this file worth reading twice. The handle
	 * rides the rail's content-facing edge — the RIGHT edge of a left-docked
	 * rail, the LEFT edge of a right-docked one — so the same rightward travel
	 * has to grow one and shrink the other. Shared by the pointer and the arrow
	 * keys because otherwise the two paths ship the flip separately, and the one
	 * nobody switched the setting to test ships inverted.
	 */
	function widthAfter(dx: number, from: number): number {
		return normaliseWidth(from + (side === 'left' ? dx : -dx));
	}

	function handlePointerDown(event: PointerEvent<HTMLDivElement>): void {
		// Without capture the rail stops following the cursor the moment it
		// leaves the strip, which at seven pixels wide is the first frame.
		event.currentTarget.setPointerCapture(event.pointerId);
		drag.current = {
			originX: event.clientX,
			originWidth: width,
			shell: shellOf(event.currentTarget),
		};
		setDragging(true);
	}

	function handlePointerMove(event: PointerEvent<HTMLDivElement>): void {
		const active = drag.current;
		if (active === null) return;
		paint(active.shell, widthAfter(event.clientX - active.originX, active.originWidth));
	}

	function handlePointerUp(event: PointerEvent<HTMLDivElement>): void {
		const active = drag.current;
		if (active === null) return;
		drag.current = null;
		setDragging(false);
		event.currentTarget.releasePointerCapture(event.pointerId);
		commit(widthAfter(event.clientX - active.originX, active.originWidth));
	}

	/**
	 * Capture can end without a `pointerup`: the window loses focus, the OS takes
	 * the pointer, a touch is cancelled. A drag that outlives its capture turns
	 * the next hover across the handle into a resize with no button held.
	 */
	function handleLostPointerCapture(): void {
		drag.current = null;
		setDragging(false);
	}

	function handleKeyDown(event: KeyboardEvent<HTMLDivElement>): void {
		const dx = event.key === 'ArrowLeft' ? -NUDGE : event.key === 'ArrowRight' ? NUDGE : 0;
		if (dx === 0) return;

		// Left and right scroll the rail's own list otherwise, and the focus ring
		// wanders off the handle mid-resize.
		event.preventDefault();

		const next = widthAfter(dx, width);
		paint(shellOf(event.currentTarget), next);
		commit(next);
	}

	return (
		<div
			role="separator"
			aria-orientation="vertical"
			aria-label={t('sidebar.resize')}
			// The committed width, which is a frame behind during a drag on
			// purpose: re-rendering per `pointermove` to keep it exact is the
			// render storm the ref exists to avoid, and nothing is listening to
			// this attribute while a pointer is down. The keyboard path commits
			// on every press, so it stays exact for the users who read it.
			aria-valuenow={width}
			aria-valuemin={SIDEBAR_MIN}
			aria-valuemax={SIDEBAR_MAX}
			tabIndex={0}
			onPointerDown={handlePointerDown}
			onPointerMove={handlePointerMove}
			onPointerUp={handlePointerUp}
			onLostPointerCapture={handleLostPointerCapture}
			onKeyDown={handleKeyDown}
			data-dragging={dragging}
			className={cn(GRIP, side === 'left' ? '-right-[3px]' : '-left-[3px]')}
		/>
	);
}
