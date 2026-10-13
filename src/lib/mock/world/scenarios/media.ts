/**
 * Stand-in art for the mock world.
 *
 * Shaped by what the real daemon actually sends: a published arrow carries an
 * icon and, far more often than not, an empty banner. Fixtures that gave every
 * arrow the same banner and no icon inverted both facts, so the mock never
 * rendered the case that ships.
 *
 * Art is derived from the namespace rather than picked from a list: every arrow
 * gets its own, and a scenario can grow without anyone assigning images.
 */

function dataUri(width: number, height: number, body: string): string {
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}">${body}</svg>`;
	// `encodeURIComponent` leaves `(`/`)` unescaped, which breaks this URI when
	// dropped straight into a markdown `![]()` destination -- escape them too.
	return `data:image/svg+xml,${encodeURIComponent(svg).replace(/\(/g, '%28').replace(/\)/g, '%29')}`;
}

/**
 * djb2 plus an avalanche finalizer, for a stable pick per namespace. The
 * finalizer is not decoration: every fixture namespace shares the same long
 * prefix, and raw djb2 leaves that similarity sitting in the low bits -- which
 * are exactly the bits the palette indexes with, so half the arrows came out
 * the same colour. Nothing here is security-sensitive.
 */
function hashOf(seed: string): number {
	let hash = 5381;
	for (let i = 0; i < seed.length; i++) hash = (Math.imul(hash, 33) ^ seed.charCodeAt(i)) >>> 0;
	hash = Math.imul(hash ^ (hash >>> 16), 0x7feb352d) >>> 0;
	hash = Math.imul(hash ^ (hash >>> 15), 0x846ca68b) >>> 0;
	return (hash ^ (hash >>> 16)) >>> 0;
}

/**
 * Twenty-four hues at 15 degrees apart, each at two lightnesses. Quantising
 * rather than taking the hash modulo 360 is what keeps two arrows from landing
 * four degrees apart and reading as the same card: a collision here is either
 * the same swatch or an obviously different one.
 */
function paletteOf(namespace: string): { hue: number; lift: number } {
	const hash = hashOf(namespace);
	return { hue: (hash % 24) * 15, lift: ((hash >>> 8) % 2) * 9 };
}

/** The quiver mark: a chevron over a dot, scaled to a 512 box. */
const MARK =
	'<path d="M144 368 L256 152 L368 368" fill="none" stroke="#fff" stroke-opacity="0.92" stroke-width="34" stroke-linecap="round" stroke-linejoin="round"/>' +
	'<circle cx="256" cy="392" r="26" fill="#fff" fill-opacity="0.92"/>';

export function iconFor(namespace: string): string {
	const { hue, lift } = paletteOf(namespace);
	return dataUri(
		512,
		512,
		`<rect width="512" height="512" rx="112" fill="hsl(${hue}, 48%, ${30 + lift}%)"/>` +
			`<rect width="512" height="256" rx="112" fill="hsl(${hue}, 52%, ${38 + lift}%)"/>` +
			MARK
	);
}

export function bannerFor(namespace: string): string {
	const { hue, lift } = paletteOf(namespace);
	return dataUri(
		1200,
		600,
		`<rect width="1200" height="600" fill="hsl(${hue}, 40%, ${17 + lift}%)"/>` +
			`<g stroke="hsl(${hue}, 40%, ${24 + lift}%)" stroke-width="2">` +
			Array.from({ length: 11 }, (_, i) => `<path d="M${i * 120} 0 L${i * 120} 600"/>`).join('') +
			'</g>' +
			`<circle cx="960" cy="120" r="260" fill="hsl(${hue}, 46%, ${23 + lift}%)"/>` +
			`<g transform="translate(344 44)">${MARK}</g>`
	);
}
