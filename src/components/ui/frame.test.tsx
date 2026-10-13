import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

import { Frame, FrameDescription, FrameFooter, FrameHeader, FramePanel, FrameTitle } from './frame';

describe('Frame', () => {
	it('renders its children inside the muted matting', () => {
		render(<Frame data-testid="frame">content</Frame>);
		const frame = screen.getByTestId('frame');
		expect(frame).toHaveTextContent('content');
		expect(frame).toHaveAttribute('data-slot', 'frame');
		expect(frame.className).toContain('bg-muted');
		expect(frame.className).toContain('rounded-2xl');
	});

	it('merges a passed className rather than replacing the default classes', () => {
		render(<Frame className="custom-frame" data-testid="frame" />);
		const frame = screen.getByTestId('frame');
		expect(frame.className).toContain('custom-frame');
		expect(frame.className).toContain('bg-muted');
	});
});

describe('FramePanel', () => {
	it('renders its children on the card surface', () => {
		render(<FramePanel data-testid="panel">panel content</FramePanel>);
		const panel = screen.getByTestId('panel');
		expect(panel).toHaveTextContent('panel content');
		expect(panel).toHaveAttribute('data-slot', 'frame-panel');
	});

	it('gets the rounded, bordered card classes', () => {
		render(<FramePanel data-testid="panel" />);
		const panel = screen.getByTestId('panel');
		expect(panel.className).toContain('not-has-[table]:rounded-xl');
		expect(panel.className).toContain('not-has-[table]:border');
		expect(panel.className).toContain('not-has-[table]:bg-card');
		expect(panel.className).toContain('not-has-[table]:shadow-xs');
	});

	it('merges a passed className rather than replacing the default classes', () => {
		render(<FramePanel className="custom-panel" data-testid="panel" />);
		const panel = screen.getByTestId('panel');
		expect(panel.className).toContain('custom-panel');
		expect(panel.className).toContain('not-has-[table]:bg-card');
	});
});

describe('FrameHeader', () => {
	it('renders its children and is not nested inside a panel by itself', () => {
		render(<FrameHeader data-testid="header">header content</FrameHeader>);
		const header = screen.getByTestId('header');
		expect(header).toHaveTextContent('header content');
		expect(header).toHaveAttribute('data-slot', 'frame-panel-header');
		expect(header.tagName).toBe('HEADER');
	});

	it('merges a passed className rather than replacing the default classes', () => {
		render(<FrameHeader className="custom-header" data-testid="header" />);
		const header = screen.getByTestId('header');
		expect(header.className).toContain('custom-header');
		expect(header.className).toContain('px-5');
	});
});

describe('FrameTitle', () => {
	it('renders its children', () => {
		render(<FrameTitle>My Arrow</FrameTitle>);
		expect(screen.getByText('My Arrow')).toBeInTheDocument();
	});

	it('gets the semibold heading classes', () => {
		render(<FrameTitle data-testid="title">My Arrow</FrameTitle>);
		const title = screen.getByTestId('title');
		expect(title.className).toContain('font-semibold');
		expect(title.className).toContain('text-sm');
	});

	it('merges a passed className rather than replacing the default classes', () => {
		render(
			<FrameTitle className="custom-title" data-testid="title">
				My Arrow
			</FrameTitle>
		);
		const title = screen.getByTestId('title');
		expect(title.className).toContain('custom-title');
		expect(title.className).toContain('font-semibold');
	});
});

describe('FrameDescription', () => {
	it('renders its children', () => {
		render(<FrameDescription>Some description</FrameDescription>);
		expect(screen.getByText('Some description')).toBeInTheDocument();
	});

	it('gets the muted foreground classes', () => {
		render(<FrameDescription data-testid="description">Some description</FrameDescription>);
		const description = screen.getByTestId('description');
		expect(description.className).toContain('text-muted-foreground');
		expect(description.className).toContain('text-sm');
	});

	it('merges a passed className rather than replacing the default classes', () => {
		render(
			<FrameDescription className="custom-description" data-testid="description">
				Some description
			</FrameDescription>
		);
		const description = screen.getByTestId('description');
		expect(description.className).toContain('custom-description');
		expect(description.className).toContain('text-muted-foreground');
	});
});

describe('FrameFooter', () => {
	it('renders its children', () => {
		render(<FrameFooter data-testid="footer">footer content</FrameFooter>);
		const footer = screen.getByTestId('footer');
		expect(footer).toHaveTextContent('footer content');
		expect(footer).toHaveAttribute('data-slot', 'frame-panel-footer');
		expect(footer.tagName).toBe('FOOTER');
	});

	it('merges a passed className rather than replacing the default classes', () => {
		render(<FrameFooter className="custom-footer" data-testid="footer" />);
		const footer = screen.getByTestId('footer');
		expect(footer.className).toContain('custom-footer');
		expect(footer.className).toContain('px-5');
	});
});
