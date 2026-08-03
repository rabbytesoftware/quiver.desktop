// Drawn rather than shipped: the mock is in release builds, so a pair of PNGs
// at the real dimensions would be tens of kilobytes every user downloads.
//
// Most arrows here declare no media on purpose — all eleven demo manifests in
// quiver.core/docs/templates/demo declare none either.

function dataUri(width: number, height: number, body: string): string {
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}">${body}</svg>`;
	return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** 512×512, the icon shape a manifest declares. */
export const DEMO_ICON = dataUri(
	512,
	512,
	'<rect width="512" height="512" fill="#111"/>' +
		'<path d="M128 384 L256 128 L384 384 Z" fill="none" stroke="#eee" stroke-width="24"/>' +
		'<circle cx="256" cy="300" r="36" fill="#eee"/>'
);

/** 2:1 at 1200×600, subject inside the middle 60% so a narrower crop keeps it. */
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
