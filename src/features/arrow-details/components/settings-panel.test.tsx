import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { ArrowVariable } from '@/domain/arrow';

import { SettingsPanel } from './settings-panel';

const BASE_VARIABLES: ArrowVariable[] = [
	{ name: 'server-name', description: 'Shown in the server list.', type: 'string', default: 'My Server' },
	{ name: 'max-players', description: 'Player cap.', type: 'number', default: '10' },
];

describe('SettingsPanel', () => {
	it('renders the title and every variable, name and description, with no dialog involved', () => {
		render(<SettingsPanel onChange={vi.fn()} values={{}} variables={BASE_VARIABLES} />);

		expect(screen.getByText('Settings')).toBeInTheDocument();
		expect(screen.getByText('server-name')).toBeInTheDocument();
		expect(screen.getByText('Shown in the server list.')).toBeInTheDocument();
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('shows the plural summary for more than one variable', () => {
		render(<SettingsPanel onChange={vi.fn()} values={{}} variables={BASE_VARIABLES} />);
		expect(screen.getByText('2 settings')).toBeInTheDocument();
	});

	it('shows the singular summary for exactly one variable', () => {
		render(<SettingsPanel onChange={vi.fn()} values={{}} variables={[BASE_VARIABLES[0]]} />);
		expect(screen.getByText('1 setting')).toBeInTheDocument();
	});

	it('shows the summary even at zero variables, rather than an empty or missing badge', () => {
		render(<SettingsPanel onChange={vi.fn()} values={{}} variables={[]} />);
		expect(screen.getByText('0 settings')).toBeInTheDocument();
	});

	it('appends the sensitive count to the summary when any variable is sensitive', () => {
		const sensitive: ArrowVariable = { name: 'rcon-password', description: '', type: 'string', sensitive: true };
		render(<SettingsPanel onChange={vi.fn()} values={{}} variables={[...BASE_VARIABLES, sensitive]} />);
		expect(screen.getByText('3 settings · 1 sensitive')).toBeInTheDocument();
	});

	it('calls onChange when a field is edited', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(
			<SettingsPanel onChange={onChange} values={{ 'server-name': 'My Server' }} variables={BASE_VARIABLES} />
		);

		const field = screen.getByRole('textbox', { name: 'server-name' });
		await user.type(field, '!');

		expect(onChange).toHaveBeenCalledWith('server-name', 'My Server!');
	});
});
