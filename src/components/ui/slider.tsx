import { Slider as SliderPrimitive } from '@base-ui/react/slider';

import { cn } from '@/lib/cn';

function Slider({
	className,
	defaultValue,
	value,
	min = 0,
	max = 100,
	// Forwarded to the THUMB, which is what carries role="slider" — on the root
	// it labels an element no assistive tech reports as a slider.
	'aria-label': ariaLabel,
	...props
}: SliderPrimitive.Root.Props & { 'aria-label'?: string }) {
	// A SCALAR value means one thumb. As generated this fell through to
	// `[min, max]` and rendered two, turning every single-value slider into a
	// range — and Base UI then assigns no `role="slider"` at all, so the control
	// is invisible to assistive tech and to any test that looks for it.
	const _values = Array.isArray(value)
		? value
		: Array.isArray(defaultValue)
			? defaultValue
			: value !== undefined || defaultValue !== undefined
				? [0]
				: [min, max];

	return (
		<SliderPrimitive.Root
			className={cn('data-horizontal:w-full data-vertical:h-full', className)}
			data-slot="slider"
			defaultValue={defaultValue}
			value={value}
			min={min}
			max={max}
			thumbAlignment="edge"
			{...props}
		>
			<SliderPrimitive.Control className="relative flex w-full touch-none items-center select-none data-disabled:opacity-50 data-vertical:h-full data-vertical:min-h-40 data-vertical:w-auto data-vertical:flex-col">
				<SliderPrimitive.Track
					data-slot="slider-track"
					className="relative grow overflow-hidden bg-muted-foreground/30 select-none data-horizontal:h-1.5 data-horizontal:w-full data-vertical:h-full data-vertical:w-1.5"
				>
					<SliderPrimitive.Indicator
						data-slot="slider-range"
						className="bg-primary select-none data-horizontal:h-full data-vertical:w-full"
					/>
				</SliderPrimitive.Track>
				{Array.from({ length: _values.length }, (_, index) => (
					<SliderPrimitive.Thumb
						data-slot="slider-thumb"
						key={index}
						aria-label={ariaLabel}
						className="block h-4 w-1.5 shrink-0 border border-primary bg-primary shadow-sm ring-ring/50 transition-[color,box-shadow] select-none hover:ring-4 focus-visible:ring-4 focus-visible:outline-hidden disabled:pointer-events-none disabled:opacity-50"
					/>
				))}
			</SliderPrimitive.Control>
		</SliderPrimitive.Root>
	);
}

export { Slider };
