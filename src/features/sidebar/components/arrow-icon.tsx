import type { JSX } from 'react';

import { useTranslation } from '@/lib/i18n';

/**
 * The chip's hue, derived from the arrow's namespace.
 *
 * DERIVED rather than looked up or rolled, because the other two both fail on
 * an arrow this file has never seen. A table only colours the arrows someone
 * remembered to add to it, so every arrow published afterwards falls through to
 * one shared default and the rail goes back to being the column of identical
 * grey placeholders this replaces. A random pick colours all of them and none
 * of them: the colour changes on every mount and differs between two open
 * windows, so it stops being the thing that finds the row you were on a moment
 * ago. A hash is the only one of the three that is both total and stable.
 *
 * Keyed on the namespace and not the name: the namespace is what the store keys
 * on and what survives a manifest getting a nicer title, so an arrow does not
 * change colour when it is renamed.
 */
function chipHue(namespace: string): number {
	// FNV-1a. `Math.imul` rather than `*`: the 32-bit product leaves the range
	// the number type holds exactly, and the low bits — the only ones the hue is
	// taken from — come back rounded to zeros, which would collapse whole
	// families of namespaces onto one colour.
	let hash = 0x811c9dc5;
	for (let index = 0; index < namespace.length; index += 1) {
		hash = Math.imul(hash ^ namespace.charCodeAt(index), 0x01000193);
	}
	return (hash >>> 0) % 360;
}

/**
 * Lightness and chroma are fixed and the hue is all that moves, so white
 * 700-weight text clears WCAG AA on all 360 of them — 4.70:1 at the worst hue
 * and 5.99:1 at the best.
 *
 * OKLCH rather than HSL, which cannot make that promise from one pair of
 * numbers: its L is a channel average, so `hsl(60 58% 42%)` is 2.5:1 against
 * white while `hsl(240 58% 42%)` is 13:1, and every yellow-ish chip would be
 * unreadable while every blue one turned nearly black. OKLCH's L is perceptual,
 * which is also why index.css is written in it.
 *
 * The chip carries its own background rather than tinting the row's, so it does
 * not care what it is sitting on: it reads the same on the rail, on the
 * inverted selected row, and in either theme.
 */
function chipColour(namespace: string): string {
	return `oklch(0.52 0.15 ${chipHue(namespace)})`;
}

/**
 * Two glyphs: initials where the name has words to take them from, the opening
 * pair where it does not.
 *
 * `Array.from` rather than `charAt` or `slice` — a name that opens with an
 * emoji hands either of those back half a surrogate pair, which paints as a
 * replacement box in a chip with room for exactly two glyphs.
 */
function monogram(name: string): string {
	const words = name.split(/\s+/).filter((word) => word.length > 0);
	const glyphs = words.length > 1 ? words.map((word) => Array.from(word)[0]) : Array.from(words[0] ?? '');
	return glyphs.slice(0, 2).join('');
}

/** The `--icon` slot itself, so a lettered row and an imaged one line their labels up. */
const TILE = 'grid size-(--icon) shrink-0 place-items-center';

/**
 * `leading` set to the chip's own height, not `place-items` on the parent: the
 * grid centres the box, and only the line box centres the text inside it. Left
 * to the row's inherited 1.25 the two capitals sit a pixel or so high in an
 * 18px square, which every chip in the column repeats.
 *
 * `tracking-[0]` because the row sets -0.1px for its 13px label, and the same
 * negative tracking on two 8.5px capitals reads as a kerning fault.
 */
const MONOGRAM = [
	'block size-[calc(var(--icon)_-_2px)] text-center text-[8.5px]',
	'leading-[calc(var(--icon)_-_2px)] font-[700] tracking-[0] text-white uppercase',
].join(' ');

interface ArrowIconProps {
	/** Keys the chip's colour — see `chipHue`. */
	namespace: string;
	name: string;
	/** `ArrowEntry.icon` — null for any arrow whose manifest ships none. */
	icon: string | null;
}

/**
 * An arrow's mark in the rail.
 *
 * The monogram chip is ours, not the design's — `design.pen` draws every row
 * with an icon and says nothing about the arrows that have none, and today that
 * is all of them. Without it those rows lose their `--icon` box entirely and
 * their labels sit one column left of every other row's.
 */
export function ArrowIcon({ namespace, name, icon }: ArrowIconProps): JSX.Element {
	const { t } = useTranslation();

	if (icon !== null) {
		// Decorative on purpose: the name is the next thing in the row, so a
		// named image would make every row announce it twice.
		return <img data-slot="arrow-icon" src={icon} alt="" className="size-(--icon) shrink-0 object-cover" />;
	}

	return (
		// Labelled where the image above is not: the monogram is not the name,
		// and two lettered chips are indistinguishable to a screen reader.
		<span data-slot="arrow-icon" role="img" aria-label={t('arrow.icon.fallback', { name })} className={TILE}>
			<span data-slot="arrow-monogram" className={MONOGRAM} style={{ backgroundColor: chipColour(namespace) }}>
				{monogram(name)}
			</span>
		</span>
	);
}
