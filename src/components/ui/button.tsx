import type { ButtonHTMLAttributes } from 'react';

import { cn } from '@/lib/cn';

type Variant = 'default' | 'ghost' | 'outline';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
	variant?: Variant;
}

const BASE =
	'inline-flex h-[26px] shrink-0 select-none items-center justify-center gap-1.5 px-2.5 ' +
	'text-[13px] leading-none transition-colors ' +
	'focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-1 focus-visible:outline-ring ' +
	'disabled:pointer-events-none disabled:opacity-40';

const VARIANTS: Record<Variant, string> = {
	default: 'bg-fill text-fill-ink hover:opacity-90',
	ghost: 'text-ink-2 hover:bg-hover hover:text-ink',
	outline: 'border border-line text-ink hover:bg-hover',
};

export function Button({ variant = 'ghost', className, type = 'button', ...rest }: ButtonProps) {
	return <button type={type} className={cn(BASE, VARIANTS[variant], className)} {...rest} />;
}
