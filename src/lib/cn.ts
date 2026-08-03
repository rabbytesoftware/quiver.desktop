import { twMerge } from 'tailwind-merge';

/**
 * Class list, last-writer-wins on conflicts.
 *
 * `twMerge` alone rather than `clsx` + `twMerge`: every call site here passes
 * strings and the odd `false &&`, and filtering those is two lines rather than
 * a dependency.
 */
export function cn(...parts: Array<string | false | null | undefined>): string {
	return twMerge(parts.filter(Boolean).join(' '));
}
