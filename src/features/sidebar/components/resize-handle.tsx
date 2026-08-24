/* eslint-disable jsx-a11y/no-noninteractive-element-interactions, jsx-a11y/no-noninteractive-tabindex */

import { useRef, useState, type JSX, type KeyboardEvent, type PointerEvent } from 'react';

import { normaliseWidth, SIDEBAR_MAX, SIDEBAR_MIN, useShellStore } from '@/features/shell/stores/shell-store';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

const NUDGE = 8;

interface Drag {
	originX: number;
	originWidth: number;
	shell: HTMLElement;
}

function shellOf(handle: HTMLElement): HTMLElement {
	return handle.closest<HTMLElement>('[data-shell], [style*="--rail"]') ?? document.documentElement;
}

function paint(shell: HTMLElement, width: number): void {
	shell.style.setProperty('--rail', `${width}px`);
}

const GRIP = [
	'absolute inset-y-0 z-10 w-[7px] cursor-col-resize touch-none focus-visible:outline-2',
	"after:absolute after:inset-y-0 after:left-[3px] after:w-px after:content-['']",
	'after:bg-muted-foreground after:opacity-0',
	'hover:after:opacity-100 data-[dragging=true]:after:opacity-100',
].join(' ');

export function ResizeHandle(): JSX.Element {
	const { t } = useTranslation();
	const side = useShellStore((s) => s.sidebarSide);
	const width = useShellStore((s) => s.sidebarWidth);
	const commit = useShellStore((s) => s.setSidebarWidth);

	const drag = useRef<Drag | null>(null);

	const [dragging, setDragging] = useState(false);

	function widthAfter(dx: number, from: number): number {
		return normaliseWidth(from + (side === 'left' ? dx : -dx));
	}

	function handlePointerDown(event: PointerEvent<HTMLDivElement>): void {
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

	function handleLostPointerCapture(): void {
		drag.current = null;
		setDragging(false);
	}

	function handleKeyDown(event: KeyboardEvent<HTMLDivElement>): void {
		const dx = event.key === 'ArrowLeft' ? -NUDGE : event.key === 'ArrowRight' ? NUDGE : 0;
		if (dx === 0) return;

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
