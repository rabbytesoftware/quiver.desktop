import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { InferenceNotes } from './inference-notes';

describe('InferenceNotes', () => {
	it('renders nothing without confidence or warnings', () => {
		const { container } = render(<InferenceNotes confidence={null} warnings={[]} />);
		expect(container).toBeEmptyDOMElement();
	});

	it('shows the confidence alone', () => {
		render(<InferenceNotes confidence="high" warnings={[]} />);
		expect(screen.getByText('Confidence: high')).toBeInTheDocument();
		expect(screen.queryByText('Things to check')).not.toBeInTheDocument();
	});

	it('translates known warning codes and shows unknown ones raw', () => {
		render(<InferenceNotes confidence={undefined} warnings={['assumed_arch', 'brand_new_code']} />);
		expect(screen.getByText('The CPU architecture was assumed, not stated by the release.')).toBeInTheDocument();
		expect(screen.getByText('brand_new_code')).toBeInTheDocument();
	});
});
