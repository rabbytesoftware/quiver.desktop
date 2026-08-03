// The Base UI controls the settings panels need, wearing this design's tokens.
//
// Adopted rather than hand-rolled because each one carries behaviour that is
// tedious to get right and invisible when wrong: Switch owns the checkbox
// semantics and the disabled/focus states; Select owns typeahead, roving focus
// and the collision-aware positioner; Slider owns keyboard stepping and the
// pointer-capture drag; NumberField owns the "type 1e9 into a number input"
// problem. What is written here is only how they look.

import { cn } from '@/lib/cn';

import { NumberField as BaseNumberField } from '@base-ui-components/react/number-field';
import { Select as BaseSelect } from '@base-ui-components/react/select';
import { Slider as BaseSlider } from '@base-ui-components/react/slider';
import { Switch as BaseSwitch } from '@base-ui-components/react/switch';

// ── Switch ───────────────────────────────────────────────────────────────────

export function Switch({
	checked,
	onCheckedChange,
	disabled,
	'aria-label': ariaLabel,
}: {
	checked: boolean;
	onCheckedChange: (checked: boolean) => void;
	disabled?: boolean;
	'aria-label'?: string;
}) {
	return (
		<BaseSwitch.Root
			checked={checked}
			onCheckedChange={onCheckedChange}
			disabled={disabled}
			aria-label={ariaLabel}
			className={cn(
				'relative h-[18px] w-[32px] shrink-0 border border-line bg-field transition-colors',
				'focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-1 focus-visible:outline-ring',
				'data-[checked]:border-fill data-[checked]:bg-fill',
				'disabled:opacity-40'
			)}
		>
			<BaseSwitch.Thumb
				className={cn(
					'block h-[12px] w-[12px] translate-x-[2px] bg-ink-2 transition-transform',
					'data-[checked]:translate-x-[16px] data-[checked]:bg-fill-ink'
				)}
			/>
		</BaseSwitch.Root>
	);
}

// ── Select ───────────────────────────────────────────────────────────────────

export interface SelectOption<T extends string> {
	value: T;
	label: string;
}

export function Select<T extends string>({
	value,
	onValueChange,
	options,
	className,
	'aria-label': ariaLabel,
}: {
	value: T;
	onValueChange: (value: T) => void;
	options: Array<SelectOption<T>>;
	className?: string;
	'aria-label'?: string;
}) {
	return (
		// `items` is what lets `Select.Value` render the LABEL rather than the
		// raw value — without it the trigger shows `normal` where it should show
		// `Normal`, and any value that is a slug shows the slug.
		<BaseSelect.Root items={options} value={value} onValueChange={(v) => onValueChange(v as T)}>
			<BaseSelect.Trigger
				aria-label={ariaLabel}
				className={cn(
					'inline-flex h-[26px] items-center justify-between gap-2 border border-line bg-field px-2 text-[13px] text-field-ink',
					'hover:bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-1 focus-visible:outline-ring',
					className
				)}
			>
				<BaseSelect.Value />
				<BaseSelect.Icon className="text-ink-3">▾</BaseSelect.Icon>
			</BaseSelect.Trigger>
			<BaseSelect.Portal>
				<BaseSelect.Positioner sideOffset={2} className="z-50">
					<BaseSelect.Popup className="min-w-[var(--anchor-width)] border border-line bg-plate py-1 shadow-lg">
						{options.map((option) => (
							<BaseSelect.Item
								key={option.value}
								value={option.value}
								className={cn(
									'flex cursor-default select-none items-center px-2 py-1 text-[13px] text-ink',
									'data-[highlighted]:bg-fill data-[highlighted]:text-fill-ink'
								)}
							>
								<BaseSelect.ItemText>{option.label}</BaseSelect.ItemText>
							</BaseSelect.Item>
						))}
					</BaseSelect.Popup>
				</BaseSelect.Positioner>
			</BaseSelect.Portal>
		</BaseSelect.Root>
	);
}

// ── Slider ───────────────────────────────────────────────────────────────────

export function Slider({
	value,
	onValueChange,
	min = 0,
	max = 100,
	step = 1,
	className,
	'aria-label': ariaLabel,
}: {
	value: number;
	onValueChange: (value: number) => void;
	min?: number;
	max?: number;
	step?: number;
	className?: string;
	'aria-label'?: string;
}) {
	return (
		<BaseSlider.Root
			value={value}
			// Base UI hands back an array for range sliders and a number for single
			// ones; normalising here keeps every caller on the number.
			onValueChange={(next) => onValueChange(Array.isArray(next) ? (next[0] ?? 0) : next)}
			min={min}
			max={max}
			step={step}
			className={cn('flex items-center', className)}
		>
			<BaseSlider.Control className="flex h-[18px] w-full items-center">
				<BaseSlider.Track className="h-[3px] w-full bg-pill">
					<BaseSlider.Indicator className="h-full bg-fill" />
					<BaseSlider.Thumb
						aria-label={ariaLabel}
						className={cn(
							'h-[12px] w-[6px] bg-fill',
							'focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-1 focus-visible:outline-ring'
						)}
					/>
				</BaseSlider.Track>
			</BaseSlider.Control>
		</BaseSlider.Root>
	);
}

// ── Number field ─────────────────────────────────────────────────────────────

export function NumberField({
	value,
	onValueChange,
	min,
	max,
	step = 1,
	suffix,
	className,
	'aria-label': ariaLabel,
}: {
	value: number;
	onValueChange: (value: number) => void;
	min?: number;
	max?: number;
	step?: number;
	suffix?: string;
	className?: string;
	'aria-label'?: string;
}) {
	return (
		<BaseNumberField.Root
			value={value}
			// Clearing the field yields null, which must not become NaN downstream.
			onValueChange={(next) => onValueChange(next ?? 0)}
			min={min}
			max={max}
			step={step}
			className={cn('inline-flex items-center', className)}
		>
			<BaseNumberField.Group className="inline-flex h-[26px] items-center border border-line bg-field">
				<BaseNumberField.Decrement className="h-full w-[22px] text-ink-2 hover:bg-hover hover:text-ink">
					−
				</BaseNumberField.Decrement>
				<BaseNumberField.Input
					aria-label={ariaLabel}
					className="h-full w-[52px] bg-transparent text-center text-[13px] tabular-nums text-field-ink focus:outline-none"
				/>
				<BaseNumberField.Increment className="h-full w-[22px] text-ink-2 hover:bg-hover hover:text-ink">
					+
				</BaseNumberField.Increment>
			</BaseNumberField.Group>
			{suffix && <span className="ml-1.5 text-[12px] text-ink-3">{suffix}</span>}
		</BaseNumberField.Root>
	);
}

// ── Text input ───────────────────────────────────────────────────────────────

export function TextInput({
	value,
	onChange,
	placeholder,
	type = 'text',
	className,
	'aria-label': ariaLabel,
}: {
	value: string;
	onChange: (value: string) => void;
	placeholder?: string;
	type?: 'text' | 'password' | 'url';
	className?: string;
	'aria-label'?: string;
}) {
	return (
		<input
			type={type}
			value={value}
			onChange={(e) => onChange(e.target.value)}
			placeholder={placeholder}
			aria-label={ariaLabel}
			className={cn(
				'h-[26px] border border-line bg-field px-2 text-[13px] text-field-ink placeholder:text-field-ph',
				'focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-1 focus-visible:outline-ring',
				className
			)}
		/>
	);
}
