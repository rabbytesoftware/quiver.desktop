'use client';

import type React from 'react';

import { mergeProps } from '@base-ui/react/merge-props';
import { useRender } from '@base-ui/react/use-render';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/cn';

export const badgeVariants = cva(
	// `[&>svg]`, not `[&_svg]` (any descendant): a direct-child-only selector so
	// this never reaches into a child component's own internal svg -- e.g.
	// FlickerSpinner's inner svg relies on its own `h-full w-full` sizing to
	// stay clipped inside its 8-frame sprite strip, and a deep descendant
	// selector here would force it to a fixed square and break the animation.
	"relative inline-flex shrink-0 items-center justify-center gap-1 whitespace-nowrap rounded-sm border border-transparent font-medium outline-none transition-shadow focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-64 [&>svg:not([class*='opacity-'])]:opacity-80 [&>svg:not([class*='size-'])]:size-3.5 sm:[&>svg:not([class*='size-'])]:size-3 [&>svg]:pointer-events-none [&>svg]:shrink-0 [button&,a&]:cursor-pointer [button&,a&]:pointer-coarse:after:absolute [button&,a&]:pointer-coarse:after:size-full [button&,a&]:pointer-coarse:after:min-h-11 [button&,a&]:pointer-coarse:after:min-w-11",
	{
		defaultVariants: {
			size: 'default',
			variant: 'default',
		},
		variants: {
			size: {
				default: 'h-5.5 min-w-5.5 px-[calc(--spacing(1)-1px)] text-sm sm:h-4.5 sm:min-w-4.5 sm:text-xs',
				lg: 'h-6.5 min-w-6.5 px-[calc(--spacing(1.5)-1px)] text-base sm:h-5.5 sm:min-w-5.5 sm:text-sm',
				sm: 'h-5 min-w-5 rounded-[.25rem] px-[calc(--spacing(1)-1px)] text-xs sm:h-4 sm:min-w-4 sm:text-[.625rem]',
			},
			// The registry also ships `info`, `success`, and `warning`. They are
			// dropped rather than carried: each resolves to `--info` / `--success` /
			// `--warning`, and this theme defines none of the three. Tailwind purges
			// what nothing references, so they cost nothing today and render an
			// invisible badge the first time someone reaches for one. Add the tokens
			// to `index.css` and they can come back.
			variant: {
				default: 'bg-primary text-primary-foreground [button&,a&]:hover:bg-primary/90',
				destructive: 'bg-destructive text-white [button&,a&]:hover:bg-destructive/90',
				error: 'bg-destructive/8 text-destructive dark:bg-destructive/16',
				outline:
					'border-input bg-background text-foreground dark:bg-input/32 [button&,a&]:hover:bg-accent/50 dark:[button&,a&]:hover:bg-input/48',
				secondary: 'bg-secondary text-secondary-foreground [button&,a&]:hover:bg-secondary/90',
			},
		},
	}
);

export interface BadgeProps extends useRender.ComponentProps<'span'> {
	variant?: VariantProps<typeof badgeVariants>['variant'];
	size?: VariantProps<typeof badgeVariants>['size'];
}

export function Badge({ className, variant, size, render, ...props }: BadgeProps): React.ReactElement {
	const defaultProps = {
		className: cn(badgeVariants({ className, size, variant })),
		'data-slot': 'badge',
	};

	return useRender({
		defaultTagName: 'span',
		props: mergeProps<'span'>(defaultProps, props),
		render,
	});
}
