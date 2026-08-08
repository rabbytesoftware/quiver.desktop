import type { JSX } from 'react';

import { useTranslation } from '@/lib/i18n';

/**
 * `bg-current/10` rather than a token: the tile sits on `--sidebar` normally and
 * on `--sidebar-primary` when the row is selected, and the two invert each
 * other. A fixed colour that reads on one is invisible on the other, and
 * `currentColor` already knows which surface it is on.
 */
const TILE = 'flex size-(--icon) shrink-0 items-center justify-center bg-current/10 text-[11px] uppercase';

interface ArrowIconProps {
	name: string;
	/** `ArrowEntry.icon` — null for any arrow whose manifest ships none. */
	icon: string | null;
}

/**
 * An arrow's mark in the rail.
 *
 * The lettered fallback is ours, not the design's — `design.pen` draws every
 * row with an icon and says nothing about the arrows that have none. Without it
 * those rows lose their `--icon` box entirely and their labels sit one column
 * left of every other row's.
 */
export function ArrowIcon({ name, icon }: ArrowIconProps): JSX.Element {
	const { t } = useTranslation();

	if (icon !== null) {
		// Decorative on purpose: the name is the next thing in the row, so a
		// named image would make every row announce it twice.
		return <img data-slot="arrow-icon" src={icon} alt="" className="size-(--icon) shrink-0 object-cover" />;
	}

	// `charAt(0)` returns half a surrogate pair for a name that opens with an
	// emoji or an astral character, which paints as a replacement glyph.
	const initial = Array.from(name).slice(0, 1).join('');

	return (
		// Labelled where the image above is not: the letter is not the name, and
		// two lettered tiles are indistinguishable to a screen reader.
		<span data-slot="arrow-icon" role="img" aria-label={t('arrow.icon.fallback', { name })} className={TILE}>
			{initial}
		</span>
	);
}
