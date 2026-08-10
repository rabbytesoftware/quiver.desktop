import type { MouseEvent } from 'react';

export function blockReselect(event: MouseEvent<HTMLAnchorElement>): void {
	if (event.currentTarget.dataset.status === 'active') event.preventDefault();
}
