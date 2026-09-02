import { Fragment, useRef, type JSX } from 'react';

import { cn } from '@/lib/cn';

import {
	backspaceFocusIndex,
	clearDigit,
	distributePaste,
	nextFocusIndex,
	pasteFocusIndex,
	setDigit,
	type PairingCodeDigits,
} from '../lib/pairing-code';

export interface PairingCodeFieldProps {
	value: PairingCodeDigits;
	onChange: (next: PairingCodeDigits) => void;
	'aria-label'?: string;
}

const SLOT = cn(
	'h-[42px] w-9 shrink-0 rounded-md border border-input bg-transparent text-center font-mono text-lg font-medium text-foreground shadow-xs outline-none',
	'focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50'
);

const SLOT_KEYS = ['digit-0', 'digit-1', 'digit-2', 'digit-3', 'digit-4', 'digit-5'] as const;

export function PairingCodeField({ value, onChange, ...props }: PairingCodeFieldProps): JSX.Element {
	const refs = useRef<(HTMLInputElement | null)[]>([]);

	function focus(index: number | null) {
		if (index === null) return;
		refs.current[index]?.focus();
	}

	function handleChange(index: number, raw: string) {
		const next = setDigit(value, index, raw);
		onChange(next);
		focus(nextFocusIndex(index, next[index] !== ''));
	}

	function handleKeyDown(index: number, event: React.KeyboardEvent<HTMLInputElement>) {
		if (event.key !== 'Backspace') return;
		const target = backspaceFocusIndex(value, index);
		if (target === null) return;
		onChange(clearDigit(value, target));
		focus(target);
	}

	function handlePaste(event: React.ClipboardEvent<HTMLInputElement>) {
		const text = event.clipboardData.getData('text');
		const digits = text.replace(/[^0-9]/g, '');
		if (digits.length === 0) return;
		event.preventDefault();
		onChange(distributePaste(text));
		focus(pasteFocusIndex(digits.length));
	}

	return (
		<div aria-label={props['aria-label']} className="flex items-center gap-1.5" role="group">
			{value.map((digit, index) => (
				<Fragment key={SLOT_KEYS[index]}>
					{index === 3 && (
						<span aria-hidden="true" className="text-muted-foreground">
							–
						</span>
					)}
					<input
						aria-label={`${props['aria-label'] ?? 'Pairing code'} digit ${index + 1} of ${value.length}`}
						className={SLOT}
						inputMode="numeric"
						maxLength={1}
						onChange={(event) => handleChange(index, event.target.value)}
						onKeyDown={(event) => handleKeyDown(index, event)}
						onPaste={handlePaste}
						placeholder="•"
						ref={(el) => {
							refs.current[index] = el;
						}}
						value={digit}
					/>
				</Fragment>
			))}
		</div>
	);
}
