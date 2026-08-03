import { twMerge } from 'tailwind-merge';

/** Class list, last-writer-wins on Tailwind conflicts. */
export function cn(...parts: Array<string | false | null | undefined>): string {
	return twMerge(parts.filter(Boolean).join(' '));
}
