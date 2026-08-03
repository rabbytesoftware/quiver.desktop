import type { ButtonHTMLAttributes } from 'react';

import { cn } from '@/lib/cn';

type Variant = 'default' | 'ghost' | 'outline';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
	variant?: Variant;
}

// 26px tall, per the scale: 13px text with 6px of breathing room top and
// bottom. Not a Tailwind size step, which is why it is a literal.
const BASE =
	'inline-flex h-[26px] shrink-0 select-none items-center justify-center gap-1.5 px-2.5 ' +
	'text-[13px] leading-none transition-colors ' +
	'focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-1 focus-visible:outline-ring ' +
	'disabled:pointer-events-none disabled:opacity-40';

const VARIANTS: Record<Variant, string> = {
	// The selection idiom: a solid block with its contents knocked out.
	default: 'bg-fill text-fill-ink hover:opacity-90',
	// No border until hovered. Lines are how a monochrome interface gets noisy,
	// so they appear only where the pointer already is.
	ghost: 'text-ink-2 hover:bg-hover hover:text-ink',
	outline: 'border border-line text-ink hover:bg-hover',
};

export function Button({ variant = 'ghost', className, type = 'button', ...rest }: ButtonProps) {
	return <button type={type} className={cn(BASE, VARIANTS[variant], className)} {...rest} />;
}
