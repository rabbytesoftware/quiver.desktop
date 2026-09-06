import { describe, it, expect } from 'vitest';

import {
	backspaceFocusIndex,
	clearDigit,
	distributePaste,
	emptyPairingCode,
	isComplete,
	nextFocusIndex,
	pairingCodeValue,
	pasteFocusIndex,
	setDigit,
	type PairingCodeDigits,
} from './pairing-code';

describe('emptyPairingCode', () => {
	it('is six empty slots', () => {
		expect(emptyPairingCode()).toEqual(['', '', '', '', '', '']);
	});
});

describe('setDigit', () => {
	it('sets the digit at the given index without touching the others', () => {
		const code = setDigit(emptyPairingCode(), 2, '7');
		expect(code).toEqual(['', '', '7', '', '', '']);
	});

	it('strips non-digit characters', () => {
		expect(setDigit(emptyPairingCode(), 0, 'a')[0]).toBe('');
	});

	it('keeps only the last character of a multi-character value', () => {
		// A controlled input's onChange fires with the whole field value, so
		// typing over an existing digit briefly holds two characters.
		expect(setDigit(emptyPairingCode(), 0, '19')[0]).toBe('9');
	});
});

describe('clearDigit', () => {
	it('empties the slot at the given index', () => {
		const code = setDigit(emptyPairingCode(), 3, '5');
		expect(clearDigit(code, 3)).toEqual(['', '', '', '', '', '']);
	});
});

describe('distributePaste', () => {
	it('spreads a pasted six-digit code across all six slots in order', () => {
		expect(distributePaste('482913')).toEqual(['4', '8', '2', '9', '1', '3']);
	});

	it('strips non-digit characters before distributing', () => {
		expect(distributePaste('48-2913')).toEqual(['4', '8', '2', '9', '1', '3']);
	});

	it('truncates a pasted value longer than six digits', () => {
		expect(distributePaste('4829135')).toEqual(['4', '8', '2', '9', '1', '3']);
	});

	it('leaves trailing slots empty when fewer than six digits were pasted', () => {
		expect(distributePaste('482')).toEqual(['4', '8', '2', '', '', '']);
	});
});

describe('isComplete', () => {
	it('is false until every slot holds a digit', () => {
		expect(isComplete(['4', '8', '2', '', '', ''])).toBe(false);
	});

	it('is true once all six slots are filled', () => {
		expect(isComplete(['4', '8', '2', '9', '1', '3'])).toBe(true);
	});
});

describe('pairingCodeValue', () => {
	it('joins the slots into the plain code string', () => {
		expect(pairingCodeValue(['4', '8', '2', '9', '1', '3'])).toBe('482913');
	});
});

describe('nextFocusIndex', () => {
	it('advances to the next slot after a digit is entered', () => {
		expect(nextFocusIndex(2, true)).toBe(3);
	});

	it('does not advance past the last slot', () => {
		expect(nextFocusIndex(5, true)).toBeNull();
	});

	it('does not advance when the slot was cleared rather than filled', () => {
		expect(nextFocusIndex(2, false)).toBeNull();
	});
});

describe('backspaceFocusIndex', () => {
	it('steps back to the previous slot when the current one is already empty', () => {
		const code: PairingCodeDigits = ['4', '8', '', '', '', ''];
		expect(backspaceFocusIndex(code, 2)).toBe(1);
	});

	it('stays put when the current slot still holds a digit', () => {
		const code: PairingCodeDigits = ['4', '8', '2', '', '', ''];
		expect(backspaceFocusIndex(code, 2)).toBeNull();
	});

	it('stays put at the first slot even when empty', () => {
		expect(backspaceFocusIndex(emptyPairingCode(), 0)).toBeNull();
	});
});

describe('pasteFocusIndex', () => {
	it('focuses the slot after the last pasted digit', () => {
		expect(pasteFocusIndex(3)).toBe(3);
	});

	it('focuses the last slot when a full code was pasted', () => {
		expect(pasteFocusIndex(6)).toBe(5);
	});
});
