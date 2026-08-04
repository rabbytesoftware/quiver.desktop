import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

/** shadcn's canonical helper, kept byte-identical so pulled components work. */
export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}
