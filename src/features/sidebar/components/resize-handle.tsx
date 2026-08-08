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

import { useRef, type JSX, type KeyboardEvent, type PointerEvent } from 'react';

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
 * The rail's drag handle: a four-pixel strip on the rail's content-facing edge.
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
		// leaves the strip, which at four pixels wide is the first frame.
		event.currentTarget.setPointerCapture(event.pointerId);
		drag.current = {
			originX: event.clientX,
			originWidth: width,
			shell: shellOf(event.currentTarget),
		};
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
			className={cn(
				// Absolute, so the strip overlays the rail's edge instead of
				// taking a row of its own; `touch-none` because a touch drag is
				// otherwise cancelled by the scroll it starts.
				'absolute inset-y-0 z-10 w-1 cursor-col-resize touch-none',
				'hover:bg-sidebar-accent focus-visible:bg-sidebar-accent focus-visible:outline-2',
				side === 'left' ? 'right-0' : 'left-0'
			)}
		/>
	);
}
