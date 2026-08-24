import { describe, expect, it } from 'vitest';

import { ROW_ACTIVE, ROW_BASE, ROW_GLYPH_BOX, ROW_INACTIVE, ROW_SUBLABEL } from './row-base';

describe('row-base', () => {
	it('puts a border on every row, so selection never reflows the list', () => {
		expect(ROW_BASE).toContain('border');
		expect(ROW_INACTIVE).toContain('border-transparent');
	});

	it('INVERTS on selection rather than lifting', () => {
		expect(ROW_ACTIVE).toContain('bg-foreground');
		expect(ROW_ACTIVE).toContain('text-background');
		expect(ROW_ACTIVE).not.toContain('bg-background');
	});

	it('takes the selected row’s top edge from a token, not a literal', () => {
		expect(ROW_ACTIVE).toContain('inset-shadow-[0_1px_var(--selected-edge)]');
		expect(ROW_ACTIVE).not.toContain('white');
	});

	it('matches the fill with the border rather than contrasting it', () => {
		expect(ROW_ACTIVE).toContain('border-foreground');
	});

	it('keeps the 36px row and its 13px medium label', () => {
		expect(ROW_BASE).toContain('h-9');
		expect(ROW_BASE).toContain('text-[13px]');
		expect(ROW_BASE).toContain('font-medium');
	});

	it('rounds with the token rather than a literal', () => {
		expect(ROW_BASE).toContain('rounded-lg');
	});

	it('fixes the leading glyph box so labels start at one x', () => {
		expect(ROW_GLYPH_BOX).toContain('size-(--icon)');
		expect(ROW_GLYPH_BOX).toContain('shrink-0');
	});

	it('sets the sublabel in mono, the one place identifiers are allowed', () => {
		expect(ROW_SUBLABEL).toContain('font-mono');
		expect(ROW_SUBLABEL).toContain('opacity-60');
		expect(ROW_SUBLABEL).not.toContain('muted-foreground');
	});
});
