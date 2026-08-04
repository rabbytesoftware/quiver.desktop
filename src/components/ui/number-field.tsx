import { NumberField as NumberFieldPrimitive } from '@base-ui/react/number-field';

import { cn } from '@/lib/cn';

/**
 * Hand-written because shadcn's registry has no number field. Styled to match
 * `input.tsx` so it does not read as a different family, and kept here so
 * `shadcn add` can replace it the day one lands.
 */
function NumberField({
	className,
	suffix,
	'aria-label': ariaLabel,
	...props
}: NumberFieldPrimitive.Root.Props & { suffix?: string; 'aria-label'?: string }) {
	return (
		<NumberFieldPrimitive.Root
			data-slot="number-field"
			className={cn('inline-flex items-center', className)}
			{...props}
		>
			<NumberFieldPrimitive.Group className="inline-flex h-8 items-center border border-input bg-transparent shadow-xs transition-[color,box-shadow] focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50">
				<NumberFieldPrimitive.Decrement className="h-full w-6 text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground">
					−
				</NumberFieldPrimitive.Decrement>
				<NumberFieldPrimitive.Input
					aria-label={ariaLabel}
					className="h-full w-14 bg-transparent text-center text-sm tabular-nums text-foreground outline-none"
				/>
				<NumberFieldPrimitive.Increment className="h-full w-6 text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground">
					+
				</NumberFieldPrimitive.Increment>
			</NumberFieldPrimitive.Group>
			{suffix && <span className="ml-1.5 text-xs text-muted-foreground">{suffix}</span>}
		</NumberFieldPrimitive.Root>
	);
}

export { NumberField };
