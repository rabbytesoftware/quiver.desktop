// Two images, drawn rather than shipped.
//
// The mock is in release builds, so every fixture byte is a byte every user
// downloads. A pair of PNGs at the real dimensions would be tens of kilobytes
// for something only a developer ever sees; these are a few hundred bytes each
// and exercise the same code path — `<img src>` with a non-null value, which is
// all the media branch of the UI actually distinguishes.
//
// Most arrows here declare NO media, which is not laziness: all eleven demo
// manifests in quiver.core/docs/templates/demo declare none either, so the
// Monogram/Plate fallback is the common case in reality and deserves to be the
// common case on screen.

function dataUri(width: number, height: number, body: string): string {
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}">${body}</svg>`;
	return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** Square, 512×512 — the icon shape an arrow manifest declares. */
export const DEMO_ICON = dataUri(
	512,
	512,
	'<rect width="512" height="512" fill="#111"/>' +
		'<path d="M128 384 L256 128 L384 384 Z" fill="none" stroke="#eee" stroke-width="24"/>' +
		'<circle cx="256" cy="300" r="36" fill="#eee"/>'
);

/**
 * 2:1 at 1200×600 — the banner ratio this design settled on, with the subject
 * inside the middle 60% so a crop to any narrower column keeps it.
 */
export const DEMO_BANNER = dataUri(
	1200,
	600,
	'<rect width="1200" height="600" fill="#151515"/>' +
		'<g stroke="#2a2a2a" stroke-width="2">' +
		Array.from({ length: 11 }, (_, i) => `<path d="M${i * 120} 0 L${i * 120} 600"/>`).join('') +
		'</g>' +
		'<rect x="360" y="180" width="480" height="240" fill="none" stroke="#e8e8e8" stroke-width="6"/>' +
		'<path d="M420 360 L540 240 L660 360" fill="none" stroke="#e8e8e8" stroke-width="6"/>'
);
