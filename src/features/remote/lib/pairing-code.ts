/** A six-digit pairing code, one character (or '') per slot -- kept as a
 *  fixed-length array rather than a plain string so a digit entered out of
 *  order (paste into slot 0, then fill slot 3) never collapses the gap
 *  between it and its neighbours. */
export type PairingCodeDigits = readonly [string, string, string, string, string, string];

const LENGTH = 6;

export function emptyPairingCode(): PairingCodeDigits {
	return ['', '', '', '', '', ''];
}

/** `raw` is a controlled input's whole field value, so it may briefly hold
 *  more than one character (typing over an existing digit) -- only the
 *  last character typed is kept, matching a native single-digit field. */
export function setDigit(code: PairingCodeDigits, index: number, raw: string): PairingCodeDigits {
	const digit = raw.replace(/[^0-9]/g, '').slice(-1);
	const next = [...code] as [string, string, string, string, string, string];
	next[index] = digit;
	return next;
}

export function clearDigit(code: PairingCodeDigits, index: number): PairingCodeDigits {
	return setDigit(code, index, '');
}

export function distributePaste(pasted: string): PairingCodeDigits {
	const digits = pasted
		.replace(/[^0-9]/g, '')
		.slice(0, LENGTH)
		.split('');
	const next = emptyPairingCode() as [string, string, string, string, string, string];
	digits.forEach((digit, index) => {
		next[index] = digit;
	});
	return next;
}

export function isComplete(code: PairingCodeDigits): boolean {
	return code.every((digit) => digit !== '');
}

export function pairingCodeValue(code: PairingCodeDigits): string {
	return code.join('');
}

/** Which slot should receive focus after typing a character into `index`,
 *  or `null` to leave focus where it is. */
export function nextFocusIndex(index: number, digitEntered: boolean): number | null {
	if (digitEntered && index < LENGTH - 1) return index + 1;
	return null;
}

/** Which slot Backspace should move focus to, or `null` to let the browser's
 *  normal in-place clear happen. Only fires when the current slot is
 *  already empty -- a filled slot clears itself first, same as any native
 *  text field. */
export function backspaceFocusIndex(code: PairingCodeDigits, index: number): number | null {
	if (code[index] === '' && index > 0) return index - 1;
	return null;
}

/** Which slot to focus after a paste of `digitCount` digits. */
export function pasteFocusIndex(digitCount: number): number {
	return Math.min(digitCount, LENGTH - 1);
}
