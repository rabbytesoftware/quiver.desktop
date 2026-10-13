import { defaultSchema, type Options } from 'rehype-sanitize';

/**
 * GFM's default sanitize schema, plus `<video>`/`<audio>` (markdown has no
 * native syntax for either). No `iframe`, no `autoplay` -- ARROW.md content
 * is untrusted and archer-authored.
 */
export const readmeSanitizeSchema: Options = {
	...defaultSchema,
	tagNames: [...(defaultSchema.tagNames ?? []), 'video', 'audio', 'track'],
	attributes: {
		...defaultSchema.attributes,
		video: ['src', 'controls', 'poster', 'width', 'height', 'loop', 'muted', 'preload'],
		audio: ['src', 'controls', 'loop', 'muted', 'preload'],
		source: [...(defaultSchema.attributes?.source ?? []), 'src', 'type', 'media'],
		track: ['src', 'kind', 'srcLang', 'label', 'default'],
	},
	// `img`/`video`/`audio`/`source` all key off this one `src` allowlist -- add `data`
	// alongside http/https so an inline base64 image (a legitimate, hostless embed) isn't dropped.
	protocols: {
		...defaultSchema.protocols,
		src: [...(defaultSchema.protocols?.src ?? []), 'data'],
	},
};
