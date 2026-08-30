import type { CSSProperties, JSX } from 'react';

import { Link } from '@tanstack/react-router';

import type { SearchEntry, SearchProvenance } from '@/domain/search';
import { isHeld } from '@/domain/search';
import { cn } from '@/lib/cn';
import { cssUrl } from '@/lib/css';
import { useTranslation } from '@/lib/i18n';
import { ownerOf } from '@/lib/namespace';

import '@/features/search/styles/card.css';

/**
 * The cell is the link, so the caption is part of the same hit target and the
 * hover that lifts the banner covers both. No ground of its own and no clip: on
 * hover the card paints over whatever is above it rather than being masked by
 * its cell.
 */
const CELL = [
	'group relative block min-w-0 cursor-pointer',
	'hover:z-[2] focus-visible:z-[2]',
	'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground',
].join(' ');

/**
 * The lift distance and the strip height at once, so the banner can never
 * uncover more or less than the strip fills. Task 7's grid padding is derived
 * from this -- see spec 8.2 and 9.2.
 */
const CARD = 'relative block aspect-[2/1] min-w-0 [--reveal:30px]';

const INFO = [
	'absolute inset-x-0 bottom-0 flex items-center gap-1.5 px-0.5',
	'h-[var(--reveal)] min-w-0 text-foreground',
	// Gone (90ms) before the rebound starts (~117ms), or the kick flashes half
	// a name back into view. Spec 8.6.
	'opacity-0 transition-opacity duration-[90ms]',
	'group-hover:opacity-100 group-hover:duration-[40ms]',
	'group-focus-visible:opacity-100 group-focus-visible:duration-[40ms]',
].join(' ');

/**
 * Only a real banner becomes a background image now. The icon-only case is the
 * common one (spec 8.1.1: of the arrows reachable through discovery, every one
 * carries an icon and none carries a banner), and painting a 36px mark on flat
 * grey got worse the larger the tile grew. `card.css` draws that case instead --
 * same span, same layer, same transform target.
 */
function bannerStyle(entry: SearchEntry): CSSProperties | undefined {
	return entry.banner ? { backgroundImage: cssUrl(entry.banner) } : undefined;
}

/** The description is what the caption is for; the host is the fallback. */
function subtitle(entry: SearchEntry): string {
	return entry.description.trim() === '' ? ownerOf(entry.namespace) : entry.description;
}

/**
 * `compatible_os` is a list of os/arch pairs (`linux/amd64`), so the family is
 * the half before the slash. Advisory either way -- spec 1.1.
 */
const OS_FAMILIES: readonly { family: string; letter: string }[] = [
	{ family: 'linux', letter: 'L' },
	{ family: 'darwin', letter: 'M' },
	{ family: 'windows', letter: 'W' },
];

function families(compatible: string[]): Set<string> {
	return new Set(compatible.map((value) => value.split('/')[0]));
}

/**
 * Absent provenance on a held arrow means the stream said so: discovery knows
 * the catalog holds it but not which provenance the catalog recorded, so it
 * sends none rather than claiming the wrong one.
 */
function heldAs(entry: SearchEntry): SearchProvenance {
	return entry.provenance ?? 'installed';
}

/** One per locale, not one per card: the grid renders up to `SEARCH_LIMIT` of them. */
const COMPACT = new Map<string, Intl.NumberFormat>();

function compact(locale: string): Intl.NumberFormat {
	const cached = COMPACT.get(locale);
	if (cached) return cached;
	const formatter = new Intl.NumberFormat(locale, { notation: 'compact' });
	COMPACT.set(locale, formatter);
	return formatter;
}

export function ArrowCard({ entry }: { entry: SearchEntry }): JSX.Element {
	const { t, locale } = useTranslation();

	const held = isHeld(entry);
	// Refs arrive sorted ascending, so the last is the newest Quiver knows of.
	const version = entry.versions[entry.versions.length - 1];
	const os = families(entry.compatible_os);
	const stars = compact(locale).format(entry.stars);

	return (
		<Link
			className={CELL}
			data-slot="arrow-card"
			{...(entry.provenance ? { 'data-provenance': entry.provenance } : {})}
			params={{ _splat: entry.namespace }}
			to="/arrow/$"
		>
			<span className={CARD}>
				<span
					aria-hidden="true"
					className="absolute inset-0 overflow-hidden rounded-lg bg-muted bg-cover bg-center"
					data-slot="card-banner"
					style={bannerStyle(entry)}
				>
					{!entry.banner && (
						<span data-slot="card-drawn">
							<span data-slot="drawn-ghost">{entry.name.slice(0, 1).toUpperCase()}</span>
							{entry.icon && (
								<span data-slot="drawn-mark" style={{ backgroundImage: cssUrl(entry.icon) }} />
							)}
							<span data-slot="drawn-type">
								<span data-slot="drawn-name">{entry.name}</span>
								<span data-slot="drawn-owner">{ownerOf(entry.namespace)}</span>
							</span>
						</span>
					)}
				</span>

				{/* The strip no longer repeats the name -- the caption carries it at
				    rest -- so all 30px go to what a glance cannot answer. Spec 8.7. */}
				<span className={INFO} data-slot="card-info">
					<span
						aria-hidden="true"
						className="size-5 flex-none overflow-hidden rounded-[5px] bg-cover bg-center"
						style={entry.icon ? { backgroundImage: cssUrl(entry.icon) } : undefined}
					/>
					<span className="flex min-w-0 flex-1 items-center gap-1.5 font-mono text-[9.5px]/[12px] tracking-[-0.2px]">
						<span className="truncate opacity-90">
							{held ? t(`search.provenance.${heldAs(entry)}`) : version}
							{held && version !== undefined && ` \u00b7 ${version}`}
						</span>
						{!held && (
							<span aria-hidden="true" className="flex flex-none gap-[2.5px]">
								{OS_FAMILIES.map(({ family, letter }) => (
									<span className={cn(os.has(family) ? 'opacity-85' : 'opacity-25')} key={family}>
										{letter}
									</span>
								))}
							</span>
						)}
					</span>
					<span className="flex flex-none items-center gap-[3px] font-mono text-[9.5px] opacity-60">
						<svg aria-hidden="true" className="block" height="8" viewBox="0 0 12 12" width="8">
							<path
								d="M6 1l1.4 3.1 3.4.4-2.5 2.3.7 3.3L6 8.5 3 10.1l.7-3.3L1.2 4.5l3.4-.4z"
								fill="currentColor"
							/>
						</svg>
						<span aria-hidden="true">{stars}</span>
						<span className="sr-only">{t('search.card.stars', { count: entry.stars })}</span>
					</span>
				</span>
			</span>

			{/* Spec 8.7: identity stops depending on the pointer. Reading a screenful
			    of results used to cost one hover per result. */}
			<span className="block min-w-0 px-0.5 pt-[7px]">
				<span className="block truncate text-[12.5px]/[15px] font-medium tracking-[-0.1px]">{entry.name}</span>
				<span className="mt-px block truncate text-[11px]/[14px] text-muted-foreground">{subtitle(entry)}</span>
			</span>
		</Link>
	);
}
