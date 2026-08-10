export const ROW_BASE =
	'flex cursor-pointer select-none items-center gap-1.5 rounded-lg border ' +
	'h-9 px-1.5 mx-1.5 my-0.5 text-[13px] font-medium tracking-[-0.1px] outline-none ' +
	'focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background';

export const ROW_INACTIVE =
	'not-data-[status=active]:border-transparent not-data-[status=active]:text-foreground ' +
	'not-data-[status=active]:hover:bg-accent';

export const ROW_ACTIVE =
	'data-[status=active]:border-foreground data-[status=active]:bg-foreground ' +
	'data-[status=active]:text-background data-[status=active]:shadow-xs data-[status=active]:shadow-black/10 ' +
	'data-[status=active]:inset-shadow-[0_1px_var(--selected-edge)]';

export const ROW_GLYPH_BOX = 'inline-flex size-(--icon) shrink-0 items-center justify-center';

export const ROW_SUBLABEL = 'flex min-w-0 font-mono text-[10.5px]/[13px] tabular-nums opacity-60';
