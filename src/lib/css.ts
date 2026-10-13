/**
 * Manifests come from arbitrary repositories, so a media URL is untrusted text
 * being spliced into a CSS declaration -- quoting it as a CSS string is what
 * stops a crafted URL from closing `url(` and appending declarations of its
 * own.
 */
export function cssUrl(url: string): string {
	return `url(${JSON.stringify(url)})`;
}
