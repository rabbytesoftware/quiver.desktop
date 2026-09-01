import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { ReadmePanel } from './readme-panel';

vi.mock('./mermaid-diagram', () => ({
	MermaidDiagram: ({ code }: { code: string }) => <div data-testid="mermaid-stub">{code}</div>,
}));

describe('ReadmePanel', () => {
	it('renders headings at every level', () => {
		render(<ReadmePanel readme={'# One\n\n## Two\n\n### Three\n\n#### Four\n\n##### Five\n\n###### Six'} />);
		expect(screen.getByRole('heading', { level: 1, name: 'One' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 2, name: 'Two' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 3, name: 'Three' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 4, name: 'Four' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 5, name: 'Five' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 6, name: 'Six' })).toBeInTheDocument();
	});

	it('renders an ordered list, strikethrough, bold, italic, and a rule', () => {
		render(<ReadmePanel readme={'1. First\n2. Second\n\n**bold** *italic* ~~gone~~\n\n---'} />);
		const list = screen.getByRole('list');
		expect(list.tagName).toBe('OL');
		expect(list).toHaveTextContent('First');
		expect(list).toHaveTextContent('Second');
		expect(screen.getByText('bold').tagName).toBe('STRONG');
		expect(screen.getByText('italic').tagName).toBe('EM');
		expect(screen.getByText('gone').tagName).toBe('DEL');
		expect(document.querySelector('hr')).not.toBeNull();
	});

	it('does not open an internal anchor link in a new tab', () => {
		render(<ReadmePanel readme={'[jump](#section)'} />);
		const link = screen.getByRole('link', { name: 'jump' });
		expect(link).not.toHaveAttribute('target');
		expect(link).not.toHaveAttribute('rel');
	});

	it('renders a blockquote', () => {
		render(<ReadmePanel readme={'> A note worth calling out.'} />);
		expect(screen.getByText('A note worth calling out.').closest('blockquote')).not.toBeNull();
	});

	it('renders a GFM table', () => {
		render(<ReadmePanel readme={['| Port | Protocol |', '| --- | --- |', '| 25565 | tcp |'].join('\n')} />);
		const table = screen.getByRole('table');
		expect(table).toHaveTextContent('Port');
		expect(table).toHaveTextContent('25565');
		expect(table).toHaveTextContent('tcp');
	});

	it('renders paragraphs and a bullet list', () => {
		render(<ReadmePanel readme={'A vanilla Java Edition server.\n\n- World auto-backup\n- RCON enabled'} />);
		expect(screen.getByText('A vanilla Java Edition server.')).toBeInTheDocument();
		const list = screen.getByRole('list');
		expect(list).toHaveTextContent('World auto-backup');
		expect(list).toHaveTextContent('RCON enabled');
	});

	it('renders a GFM task list as disabled checkboxes, not bullets', () => {
		render(<ReadmePanel readme={'- [ ] Set MOTD\n- [x] Read the docs'} />);
		const boxes = screen.getAllByRole('checkbox');
		expect(boxes).toHaveLength(2);
		expect(boxes[0]).toBeDisabled();
		expect(boxes[0]).not.toBeChecked();
		expect(boxes[1]).toBeChecked();
	});

	it('opens an external link in a new tab with rel=noreferrer', () => {
		render(<ReadmePanel readme={'[docs](https://example.com/docs)'} />);
		const link = screen.getByRole('link', { name: 'docs' });
		expect(link).toHaveAttribute('href', 'https://example.com/docs');
		expect(link).toHaveAttribute('target', '_blank');
		expect(link).toHaveAttribute('rel', 'noreferrer');
	});

	it('renders an image with its alt text', () => {
		render(<ReadmePanel readme={'![A server room](https://example.com/photo.png)'} />);
		expect(screen.getByRole('img', { name: 'A server room' })).toHaveAttribute(
			'src',
			'https://example.com/photo.png'
		);
	});

	it('keeps a data: URI image instead of stripping its src', () => {
		render(<ReadmePanel readme={'![Banner](data:image/svg+xml,%3Csvg%3E%3C/svg%3E)'} />);
		expect(screen.getByRole('img', { name: 'Banner' })).toHaveAttribute(
			'src',
			'data:image/svg+xml,%3Csvg%3E%3C/svg%3E'
		);
	});

	it('renders inline code and a fenced code block', () => {
		render(<ReadmePanel readme={['Set `MOTD`, then:', '', '```bash', 'echo hi', '```'].join('\n')} />);
		expect(screen.getByText('MOTD')).toBeInTheDocument();
		expect(screen.getByText('echo hi').closest('pre')).not.toBeNull();
	});

	it('delegates a ```mermaid fenced block to MermaidDiagram instead of a code block', () => {
		render(<ReadmePanel readme={['```mermaid', 'graph TD; A-->B;', '```'].join('\n')} />);
		expect(screen.getByTestId('mermaid-stub')).toHaveTextContent('graph TD; A-->B;');
	});

	it('renders a sanitized video embed from raw HTML', () => {
		const { container } = render(
			<ReadmePanel readme={'<video src="https://example.com/clip.mp4" controls></video>'} />
		);
		const video = container.querySelector('video');
		expect(video).not.toBeNull();
		expect(video).toHaveAttribute('src', 'https://example.com/clip.mp4');
	});

	it('renders a sanitized audio embed from raw HTML', () => {
		const { container } = render(
			<ReadmePanel readme={'<audio src="https://example.com/clip.mp3" controls></audio>'} />
		);
		const audio = container.querySelector('audio');
		expect(audio).not.toBeNull();
		expect(audio).toHaveAttribute('src', 'https://example.com/clip.mp3');
	});

	it('strips a raw script tag rather than rendering or running it', () => {
		const { container } = render(<ReadmePanel readme={'<script>window.pwned = true;</script>Safe text'} />);
		expect(container.querySelector('script')).toBeNull();
		expect(screen.getByText('Safe text')).toBeInTheDocument();
	});
});
