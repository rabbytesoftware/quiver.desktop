import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { PairingCodeField } from './pairing-code-field';
import { emptyPairingCode } from '../lib/pairing-code';

function slots(): HTMLInputElement[] {
	return screen.getAllByRole('textbox') as HTMLInputElement[];
}

describe('PairingCodeField', () => {
	it('renders six slots', () => {
		render(<PairingCodeField onChange={() => {}} value={emptyPairingCode()} />);
		expect(slots()).toHaveLength(6);
	});

	it('typing a digit reports the updated code and moves focus to the next slot', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<PairingCodeField onChange={onChange} value={emptyPairingCode()} />);

		await user.type(slots()[0], '4');

		expect(onChange).toHaveBeenLastCalledWith(['4', '', '', '', '', '']);
		expect(slots()[1]).toHaveFocus();
	});

	it('typing in the last slot does not try to move focus past it', async () => {
		const user = userEvent.setup();
		render(<PairingCodeField onChange={() => {}} value={['4', '8', '2', '9', '1', '']} />);

		await user.type(slots()[5], '3');

		expect(slots()[5]).toHaveFocus();
	});

	it('backspace on an empty slot clears and focuses the previous slot', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<PairingCodeField onChange={onChange} value={['4', '8', '', '', '', '']} />);

		slots()[2].focus();
		await user.keyboard('{Backspace}');

		expect(onChange).toHaveBeenLastCalledWith(['4', '', '', '', '', '']);
		expect(slots()[1]).toHaveFocus();
	});

	it('pasting a full code distributes it across every slot', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<PairingCodeField onChange={onChange} value={emptyPairingCode()} />);

		slots()[0].focus();
		await user.paste('482913');

		expect(onChange).toHaveBeenLastCalledWith(['4', '8', '2', '9', '1', '3']);
	});
});
