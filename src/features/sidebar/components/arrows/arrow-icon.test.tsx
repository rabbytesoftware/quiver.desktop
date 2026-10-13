import type { CSSProperties } from 'react';

import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

import { ArrowIcon } from './arrow-icon';

describe('ArrowIcon', () => {
	it('renders the image when an icon is given', () => {
		render(<ArrowIcon namespace="github.com/rabbyte/minecraft" name="Minecraft Server" icon="icon.png" />);
		expect(screen.getByRole('img', { hidden: true })).toBeInTheDocument();
	});

	it('falls back to a monogram when there is no icon', () => {
		render(<ArrowIcon namespace="github.com/rabbyte/minecraft" name="Minecraft Server" icon={null} />);
		expect(screen.getByText('MS')).toBeInTheDocument();
	});

	it('monograms a single-word name from its first two letters, uppercased only via CSS', () => {
		render(<ArrowIcon namespace="github.com/rabbyte/redis" name="Redis" icon={null} />);
		// The DOM text node is "Re" -- monogram() does no case conversion in JS,
		// the visual "RE" comes entirely from the `uppercase` CSS class.
		const glyph = screen.getByText('Re');
		expect(glyph).toBeInTheDocument();
		expect(glyph.className).toContain('uppercase');
	});

	it('renders an empty monogram rather than throwing when the name is blank', () => {
		const { container } = render(<ArrowIcon namespace="github.com/rabbyte/unnamed" name="" icon={null} />);
		expect(container.querySelector('[data-slot="arrow-monogram"]')).toHaveTextContent('');
	});

	it('renders correctly under a --icon override on an ancestor', () => {
		const { container } = render(
			<div style={{ '--icon': '44px' } as CSSProperties}>
				<ArrowIcon namespace="github.com/rabbyte/game-servers" name="Game Servers" icon={null} />
			</div>
		);
		expect(screen.getByText('GS')).toBeInTheDocument();
		expect(container.querySelector('[data-slot="arrow-icon"]')).toBeInTheDocument();
	});
});
