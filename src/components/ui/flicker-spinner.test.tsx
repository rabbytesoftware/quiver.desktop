import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

import { FlickerSpinner } from './flicker-spinner';

describe('FlickerSpinner', () => {
	it('announces itself as a labelled status', () => {
		render(<FlickerSpinner />);
		expect(screen.getByRole('status', { name: 'Loading' })).toBeInTheDocument();
	});

	it('is sized with a size-4 default', () => {
		render(<FlickerSpinner />);
		expect(screen.getByRole('status').className).toContain('size-4');
	});

	it('merges a passed className rather than replacing the default classes', () => {
		render(<FlickerSpinner className="size-6 custom-spinner" />);
		const root = screen.getByRole('status');
		expect(root.className).toContain('size-6');
		expect(root.className).toContain('custom-spinner');
		expect(root.className).toContain('overflow-hidden');
	});

	it('clips the reel behind an overflow-hidden box', () => {
		render(<FlickerSpinner />);
		const status = screen.getByRole('status');
		expect(status.className).toContain('overflow-hidden');
		expect(status.className).toContain('relative');
	});

	it('drives the flicker with a discrete, steps-based CSS animation on the reel', () => {
		const { container } = render(<FlickerSpinner />);
		const reel = container.querySelector('[aria-hidden="true"]');
		expect(reel).toBeInTheDocument();
		expect(reel).toHaveClass('animate-flicker-spin');
		// The reel is 8 frames wide relative to the clipped box, and the CSS
		// keyframe (defined in index.css) shifts it by translateX(-100%) in
		// steps(8) -- jsdom doesn't run real animations, so this only checks
		// the structure that mechanism depends on, not animated visual state.
		expect(reel).toHaveClass('w-[800%]');
	});

	it('sizes the inner svg with CSS only -- no width/height attributes', () => {
		const { container } = render(<FlickerSpinner />);
		const svg = container.querySelector('svg');
		expect(svg).toBeInTheDocument();
		expect(svg).not.toHaveAttribute('width');
		expect(svg).not.toHaveAttribute('height');
		expect(svg).toHaveAttribute('viewBox', '0 0 240 30');
	});

	it('bakes all 8 frames (200 dots) as static circles rather than generating them', () => {
		const { container } = render(<FlickerSpinner />);
		const circles = container.querySelectorAll('circle');
		expect(circles).toHaveLength(200);
	});
});
