import { describe, expect, it } from 'vitest';

import { ROW_ACTIVE, ROW_BASE, ROW_GLYPH_BOX, ROW_INACTIVE, ROW_SUBLABEL } from './row-base';

/**
 * These assertions exist because each one has already been got wrong once while
 * porting the rail off Crowbar's `workspace-row-base.ts`. They are cheap, and
 * every one of them guards a mistake that looks fine in a screenshot.
 */
describe('row-base', () => {
	it('puts a border on every row, so selection never reflows the list', () => {
		// The 1px is present whether or not it is visible. Drop it from the idle
		// state and every row shifts a pixel as the selection moves through them.
		expect(ROW_BASE).toContain('border');
		expect(ROW_INACTIVE).toContain('border-transparent');
	});

	it('INVERTS on selection rather than lifting', () => {
		// Crowbar raises its row toward the content column (`bg-background`),
		// which is a one-shade difference. Quiver flips instead. Getting this
		// backwards is the single change that makes the rail read as a different
		// product, so it is asserted in both directions.
		expect(ROW_ACTIVE).toContain('bg-foreground');
		expect(ROW_ACTIVE).toContain('text-background');
		expect(ROW_ACTIVE).not.toContain('bg-background');
	});

	/**
	 * The 1px edge along the top of the selected row. Crowbar hardcodes
	 * `white/16%`, which works because its selected surface is always the dark
	 * one; ours inverts per theme, so the edge is taken from --background — the
	 * opposite of the fill. That resolves to white/16% in light (Crowbar's own
	 * value, reached by derivation) and to its mirror in dark.
	 *
	 * A literal here would be invisible in exactly one theme, which is the kind
	 * of thing only a screenshot catches.
	 */
	it('takes the selected row’s top edge from a token, not a literal', () => {
		expect(ROW_ACTIVE).toContain('inset-shadow-[0_1px_var(--selected-edge)]');
		expect(ROW_ACTIVE).not.toContain('white');
	});

	it('matches the fill with the border rather than contrasting it', () => {
		// A visible outline on the active row reads as focus, which the row can
		// already be in for unrelated reasons.
		expect(ROW_ACTIVE).toContain('border-foreground');
	});

	it('keeps the 36px row and its 13px medium label', () => {
		expect(ROW_BASE).toContain('h-9');
		expect(ROW_BASE).toContain('text-[13px]');
		expect(ROW_BASE).toContain('font-medium');
	});

	it('rounds with the token rather than a literal', () => {
		// `rounded-lg` resolves to --radius. A hard-coded corner here would not
		// follow the scale the next time it moves.
		expect(ROW_BASE).toContain('rounded-lg');
	});

	it('fixes the leading glyph box so labels start at one x', () => {
		expect(ROW_GLYPH_BOX).toContain('size-(--icon)');
		expect(ROW_GLYPH_BOX).toContain('shrink-0');
	});

	it('sets the sublabel in mono, the one place identifiers are allowed', () => {
		expect(ROW_SUBLABEL).toContain('font-mono');
		// `opacity-60`, not --muted-foreground: on an inverted row the muted
		// token is on the wrong side of the flip and stops being legible.
		expect(ROW_SUBLABEL).toContain('opacity-60');
		expect(ROW_SUBLABEL).not.toContain('muted-foreground');
	});
});
