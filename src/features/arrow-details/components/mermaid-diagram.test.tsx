import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { MermaidDiagram } from './mermaid-diagram';

const { initialize, mermaidRender } = vi.hoisted(() => ({
	initialize: vi.fn(),
	mermaidRender: vi.fn(),
}));

vi.mock('mermaid', () => ({
	default: { initialize, render: mermaidRender },
}));

beforeEach(() => {
	initialize.mockReset();
	mermaidRender.mockReset();
	document.documentElement.classList.remove('dark');
});

afterEach(() => {
	document.documentElement.classList.remove('dark');
});

describe('MermaidDiagram', () => {
	// Each test uses its own diagram source -- the component caches a
	// rendered SVG by (code, theme) across mounts (see its own comment for
	// why), so reusing a string another test already rendered would hit that
	// cache and skip calling the mocked `mermaid` module entirely.

	it('renders the diagram svg once mermaid resolves', async () => {
		mermaidRender.mockResolvedValue({ svg: '<svg data-testid="rendered-diagram"></svg>' });
		render(<MermaidDiagram code="graph TD; A-->B;" />);

		await waitFor(() => expect(screen.getByTestId('rendered-diagram')).toBeInTheDocument());
		expect(mermaidRender).toHaveBeenCalledWith(expect.stringMatching(/^mermaid-/), 'graph TD; A-->B;');
	});

	it('initializes with the light theme by default', async () => {
		mermaidRender.mockResolvedValue({ svg: '<svg></svg>' });
		render(<MermaidDiagram code="graph TD; C-->D;" />);

		await waitFor(() => expect(initialize).toHaveBeenCalled());
		expect(initialize).toHaveBeenCalledWith(expect.objectContaining({ theme: 'default' }));
	});

	it('initializes with the dark theme when the document is in dark mode', async () => {
		document.documentElement.classList.add('dark');
		mermaidRender.mockResolvedValue({ svg: '<svg></svg>' });
		render(<MermaidDiagram code="graph TD; E-->F;" />);

		await waitFor(() => expect(initialize).toHaveBeenCalled());
		expect(initialize).toHaveBeenCalledWith(expect.objectContaining({ theme: 'dark' }));
	});

	it('falls back to the raw source in a code block when mermaid fails to parse it', async () => {
		mermaidRender.mockRejectedValue(new Error('Parse error on line 1'));
		render(<MermaidDiagram code="not a real diagram" />);

		expect(await screen.findByText('not a real diagram')).toBeInTheDocument();
	});

	it('serves a repeat mount of the same diagram straight from cache, without calling mermaid again', async () => {
		mermaidRender.mockResolvedValue({ svg: '<svg data-testid="cached-diagram"></svg>' });
		const first = render(<MermaidDiagram code="graph TD; G-->H;" />);
		await waitFor(() => expect(screen.getByTestId('cached-diagram')).toBeInTheDocument());
		expect(mermaidRender).toHaveBeenCalledTimes(1);

		first.unmount();
		mermaidRender.mockClear();
		render(<MermaidDiagram code="graph TD; G-->H;" />);

		expect(screen.getByTestId('cached-diagram')).toBeInTheDocument();
		expect(mermaidRender).not.toHaveBeenCalled();
	});
});
