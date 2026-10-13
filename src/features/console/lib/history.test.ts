import { describe, expect, it } from 'vitest';

import { FRESH_CURSOR, pushHistory, stepHistory } from './history';

describe('pushHistory', () => {
	it('appends a line', () => expect(pushHistory(['a'], 'b', 10)).toEqual(['a', 'b']));
	it('trims it', () => expect(pushHistory([], '  list  ', 10)).toEqual(['list']));
	it('skips blank lines', () => expect(pushHistory(['a'], '   ', 10)).toEqual(['a']));
	it('skips an immediate repeat but not an older one', () => {
		expect(pushHistory(['a', 'b'], 'b', 10)).toEqual(['a', 'b']);
		expect(pushHistory(['a', 'b'], 'a', 10)).toEqual(['a', 'b', 'a']);
	});
	it('keeps only the newest entries at the cap', () => {
		expect(pushHistory(['a', 'b', 'c'], 'd', 3)).toEqual(['b', 'c', 'd']);
	});
});

describe('stepHistory', () => {
	const history = ['one', 'two', 'three'];

	it('does nothing with no history', () => {
		expect(stepHistory([], FRESH_CURSOR, 'typing', 'up')).toEqual({ cursor: FRESH_CURSOR, text: 'typing' });
		expect(stepHistory([], FRESH_CURSOR, 'typing', 'down')).toEqual({ cursor: FRESH_CURSOR, text: 'typing' });
	});

	it('Up starts at the newest and remembers the draft', () => {
		const step = stepHistory(history, FRESH_CURSOR, 'half-typed', 'up');
		expect(step).toEqual({ cursor: { index: 2, draft: 'half-typed' }, text: 'three' });
	});

	it('Up walks back and stops at the oldest', () => {
		let step = stepHistory(history, FRESH_CURSOR, '', 'up');
		step = stepHistory(history, step.cursor, step.text, 'up');
		step = stepHistory(history, step.cursor, step.text, 'up');
		expect(step.text).toBe('one');
		step = stepHistory(history, step.cursor, step.text, 'up');
		expect(step.text).toBe('one');
	});

	it('Down walks forward and finally restores the draft', () => {
		let step = stepHistory(history, FRESH_CURSOR, 'draft', 'up');
		step = stepHistory(history, step.cursor, step.text, 'up');
		step = stepHistory(history, step.cursor, step.text, 'down');
		expect(step.text).toBe('three');
		step = stepHistory(history, step.cursor, step.text, 'down');
		expect(step).toEqual({ cursor: { index: null, draft: '' }, text: 'draft' });
	});

	it('Down with no history position leaves the text alone', () => {
		expect(stepHistory(history, FRESH_CURSOR, 'draft', 'down')).toEqual({ cursor: FRESH_CURSOR, text: 'draft' });
	});

	it('keeps the first draft across several Up presses', () => {
		let step = stepHistory(history, FRESH_CURSOR, 'draft', 'up');
		step = stepHistory(history, step.cursor, step.text, 'up');
		expect(step.cursor.draft).toBe('draft');
	});
});
