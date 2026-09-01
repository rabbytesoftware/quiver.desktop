import type { JSX, ReactNode } from 'react';

import Markdown, { defaultUrlTransform, type Components } from 'react-markdown';
import rehypeRaw from 'rehype-raw';
import rehypeSanitize from 'rehype-sanitize';
import remarkGfm from 'remark-gfm';

import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

import { MermaidDiagram } from './mermaid-diagram';
import { readmeSanitizeSchema } from '../lib/readme-sanitize-schema';

interface ReadmePanelProps {
	readme: string;
}

// react-markdown blanks any `src`/`href` whose protocol isn't in its own
// built-in allowlist before rehype-sanitize ever sees it -- `data:` isn't in
// that list, which would silently drop an inline base64 image. `key` is the
// property name (`src` for img/video/audio/source, `href` for links), so
// this only widens image-ish embeds; rehype-sanitize's own `protocols.src`
// (readme-sanitize-schema.ts) still has the final say.
function readmeUrlTransform(value: string, key: string): string {
	if (key === 'src' && /^data:/i.test(value)) return value;
	return defaultUrlTransform(value);
}

const HEADING_CLASS: Record<'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6', string> = {
	h1: 'mt-6 mb-2 first:mt-0 text-lg font-semibold',
	h2: 'mt-6 mb-2 first:mt-0 text-base font-semibold',
	h3: 'mt-5 mb-2 first:mt-0 text-sm font-semibold',
	h4: 'mt-4 mb-1.5 first:mt-0 text-sm font-semibold',
	h5: 'mt-4 mb-1.5 first:mt-0 text-xs font-semibold uppercase tracking-wide text-muted-foreground',
	h6: 'mt-4 mb-1.5 first:mt-0 text-xs font-medium uppercase tracking-wide text-muted-foreground',
};

function heading(level: keyof typeof HEADING_CLASS) {
	const Tag = level;
	return function Heading({ children }: { children?: ReactNode }): JSX.Element {
		return <Tag className={HEADING_CLASS[level]}>{children}</Tag>;
	};
}

function codeLanguage(className?: string): string | undefined {
	return /language-(\w+)/.exec(className ?? '')?.[1];
}

function TaskCheckbox({ checked }: { checked?: boolean }): JSX.Element {
	const { t } = useTranslation();
	return (
		<input
			aria-label={checked ? t('arrow.readme.taskDone') : t('arrow.readme.taskTodo')}
			checked={checked}
			className="mr-2 align-middle accent-foreground"
			disabled
			readOnly
			type="checkbox"
		/>
	);
}

const README_COMPONENTS: Components = {
	h1: heading('h1'),
	h2: heading('h2'),
	h3: heading('h3'),
	h4: heading('h4'),
	h5: heading('h5'),
	h6: heading('h6'),

	p: ({ children }) => <p className="mb-2.5 text-sm leading-relaxed text-foreground last:mb-0">{children}</p>,

	a: ({ href, children }) => {
		const external = /^https?:\/\//.test(href ?? '');
		return (
			<a
				className="text-sm underline-offset-4 hover:underline"
				href={href}
				rel={external ? 'noreferrer' : undefined}
				target={external ? '_blank' : undefined}
			>
				{children}
			</a>
		);
	},

	strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
	em: ({ children }) => <em className="italic">{children}</em>,
	del: ({ children }) => <del className="line-through">{children}</del>,
	hr: () => <hr className="my-4 border-border" />,

	ul: ({ className, children }) => (
		<ul
			className={cn(
				'mb-2.5 space-y-1 text-sm leading-relaxed last:mb-0',
				className?.includes('contains-task-list') ? 'list-none pl-0' : 'list-disc pl-5'
			)}
		>
			{children}
		</ul>
	),
	ol: ({ children }) => (
		<ol className="mb-2.5 list-decimal space-y-1 pl-5 text-sm leading-relaxed last:mb-0">{children}</ol>
	),
	li: ({ children }) => <li className="text-sm leading-relaxed">{children}</li>,
	input: ({ type, checked }) => (type === 'checkbox' ? <TaskCheckbox checked={checked} /> : null),

	blockquote: ({ children }) => (
		<blockquote className="my-3 border-l-2 border-border pl-3 text-sm text-muted-foreground">{children}</blockquote>
	),

	pre: ({ children }) => <>{children}</>,
	code: ({ className, children }) => {
		const raw = String(children).replace(/\n$/, '');
		const language = codeLanguage(className);
		const isBlock = Boolean(language) || raw.includes('\n');

		if (language === 'mermaid') return <MermaidDiagram code={raw} />;

		if (isBlock) {
			return (
				<pre className="my-3 overflow-x-auto rounded-lg bg-muted p-3">
					<code className="font-mono text-xs leading-relaxed">{raw}</code>
				</pre>
			);
		}

		return <code className="rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]">{children}</code>;
	},

	table: ({ children }) => (
		<div className="my-3 overflow-x-auto">
			<table className="w-full border-collapse text-sm">{children}</table>
		</div>
	),
	th: ({ children }) => (
		<th className="border-b border-border px-3 py-1.5 text-left font-medium text-muted-foreground">{children}</th>
	),
	td: ({ children }) => <td className="border-b border-border px-3 py-1.5">{children}</td>,

	img: ({ src, alt }) => (
		<img alt={alt} className="my-3 block h-auto max-w-full rounded-lg border border-border" src={src} />
	),
	// Captions are the archer's own `<track>`, nested in their raw HTML and forwarded here via `{...props}`.
	video: (props) => (
		// react-doctor-disable-next-line react-doctor/media-has-caption
		<video className="my-3 block max-w-full rounded-lg border border-border" controls {...props} /> // eslint-disable-line jsx-a11y/media-has-caption
	),
	audio: (props) => (
		// react-doctor-disable-next-line react-doctor/media-has-caption
		<audio className="my-3 w-full" controls {...props} /> // eslint-disable-line jsx-a11y/media-has-caption
	),
};

/**
 * The arrow's ARROW.md, rendered as real markdown -- GFM plus fenced
 * ```mermaid``` diagrams and a small, sanitized allowance for `<video>`/
 * `<audio>` embeds. Overview shows this when present, `MetaPanel` as its
 * fallback otherwise.
 */
export function ReadmePanel({ readme }: ReadmePanelProps): JSX.Element {
	return (
		<div className="max-w-3xl text-foreground">
			<Markdown
				components={README_COMPONENTS}
				rehypePlugins={[rehypeRaw, [rehypeSanitize, readmeSanitizeSchema]]}
				remarkPlugins={[remarkGfm]}
				urlTransform={readmeUrlTransform}
			>
				{readme}
			</Markdown>
		</div>
	);
}
