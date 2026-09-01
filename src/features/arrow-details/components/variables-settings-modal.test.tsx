import { useState } from 'react';

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { ArrowVariable } from '@/domain/arrow';

import { VariablesSettingsModal } from './variables-settings-modal';

const BASE_VARIABLES: ArrowVariable[] = [
	{ name: 'server-name', description: 'Shown in the server list.', type: 'string', default: 'My Server' },
	{
		name: 'difficulty',
		description: 'How hard the world is.',
		type: 'select',
		default: 'normal',
		values: ['peaceful', 'normal', 'hard'],
	},
	{ name: 'hardcore', description: 'One life only.', type: 'boolean', default: 'false' },
	{ name: 'max-players', description: 'Player cap.', type: 'number', default: '10', min: 1, max: 20 },
];

const SENSITIVE_VARIABLE: ArrowVariable = {
	name: 'rcon-password',
	description: '',
	type: 'string',
	default: '',
	sensitive: true,
};

function baseValues(): Record<string, string> {
	return {
		'server-name': 'My Server',
		difficulty: 'normal',
		hardcore: 'false',
		'max-players': '10',
		'rcon-password': 'secret',
	};
}

/**
 * The real screen owns the values -- seeded from manifest defaults, fed
 * back in on every edit. This mirrors that instead of leaving `values`
 * static, so a keystroke that lands in the middle of a multi-character type
 * (e.g. the number field) sees the field it actually left behind.
 */
function Harness({
	variables,
	onChange,
}: {
	variables: ArrowVariable[];
	onChange: (name: string, value: string) => void;
}) {
	const [values, setValues] = useState<Record<string, string>>(baseValues());
	return (
		<VariablesSettingsModal
			onChange={(name, value) => {
				setValues((current) => ({ ...current, [name]: value }));
				onChange(name, value);
			}}
			values={values}
			variables={variables}
		/>
	);
}

describe('VariablesSettingsModal', () => {
	it('renders the Configure trigger', () => {
		render(<VariablesSettingsModal onChange={vi.fn()} values={baseValues()} variables={BASE_VARIABLES} />);
		expect(screen.getByRole('button', { name: 'Configure' })).toBeInTheDocument();
	});

	it('opens the dialog and shows every variable, name and description', async () => {
		const user = userEvent.setup();
		render(<VariablesSettingsModal onChange={vi.fn()} values={baseValues()} variables={BASE_VARIABLES} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));

		expect(
			await screen.findByText(
				'Used to configure how this arrow runs. Changes apply the next time a method is called.'
			)
		).toBeInTheDocument();
		expect(screen.getByText('server-name')).toBeInTheDocument();
		expect(screen.getByText('Shown in the server list.')).toBeInTheDocument();
	});

	it('closes the dialog when Done is pressed, without a separate save call', async () => {
		const user = userEvent.setup();
		render(<VariablesSettingsModal onChange={vi.fn()} values={baseValues()} variables={BASE_VARIABLES} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		await user.click(await screen.findByRole('button', { name: 'Done' }));

		await waitFor(() => expect(screen.queryByRole('heading', { name: 'Settings' })).not.toBeInTheDocument());
	});

	it('toggles a boolean variable through the Switch', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<Harness onChange={onChange} variables={BASE_VARIABLES} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		await user.click(await screen.findByRole('switch', { name: 'hardcore' }));

		expect(onChange).toHaveBeenCalledWith('hardcore', 'true');
	});

	it('changes a select variable through its option list', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<Harness onChange={onChange} variables={BASE_VARIABLES} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		await user.click(await screen.findByRole('combobox', { name: 'difficulty' }));
		await user.click(await screen.findByRole('option', { name: 'hard' }));

		expect(onChange).toHaveBeenCalledWith('difficulty', 'hard');
	});

	it('edits a number variable and shows its min–max range as a hint', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<Harness onChange={onChange} variables={BASE_VARIABLES} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		expect(await screen.findByText('1–20')).toBeInTheDocument();

		const field = screen.getByRole('textbox', { name: 'max-players' });
		await user.clear(field);
		await user.type(field, '15');
		await user.tab();

		expect(onChange).toHaveBeenCalledWith('max-players', '15');
	});

	it('does not show a range hint when neither min nor max is set', async () => {
		const user = userEvent.setup();
		const unranged: ArrowVariable = { name: 'seed', description: '', type: 'number', default: '0' };
		render(<Harness onChange={vi.fn()} variables={[unranged]} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		await screen.findByRole('textbox', { name: 'seed' });

		expect(screen.queryByText(/–/)).not.toBeInTheDocument();
	});

	it('edits a plain string variable', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<Harness onChange={onChange} variables={BASE_VARIABLES} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		const field = await screen.findByRole('textbox', { name: 'server-name' });
		await user.type(field, '!');

		expect(onChange).toHaveBeenCalledWith('server-name', 'My Server!');
	});

	it('falls back to the empty string for a variable with no stored value and no default', async () => {
		const user = userEvent.setup();
		const bare: ArrowVariable = { name: 'note', description: 'x', type: 'string' };
		render(<VariablesSettingsModal onChange={vi.fn()} values={{}} variables={[bare]} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));

		expect(await screen.findByRole('textbox', { name: 'note' })).toHaveValue('');
	});

	it('masks a sensitive variable behind a password input and reveals it on demand', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<Harness onChange={onChange} variables={[...BASE_VARIABLES, SENSITIVE_VARIABLE]} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));

		const field = await screen.findByLabelText('rcon-password');
		expect(field).toHaveAttribute('type', 'password');
		expect(screen.getByRole('button', { name: 'Reveal' })).toBeInTheDocument();

		await user.click(screen.getByRole('button', { name: 'Reveal' }));
		expect(field).toHaveAttribute('type', 'text');
		expect(screen.getByRole('button', { name: 'Hide' })).toBeInTheDocument();

		await user.type(field, 'x');
		expect(onChange).toHaveBeenCalledWith('rcon-password', 'secretx');

		await user.click(screen.getByRole('button', { name: 'Hide' }));
		expect(field).toHaveAttribute('type', 'password');
	});

	it('falls back to the default for a select and a sensitive field with no stored value, and to empty when neither is set', async () => {
		const user = userEvent.setup();
		const noValueVariables: ArrowVariable[] = [
			{ name: 'mode', description: '', type: 'select', default: 'normal', values: ['easy', 'normal', 'hard'] },
			{ name: 'unset-select', description: '', type: 'select' },
			{ name: 'unset-secret', description: '', type: 'string', sensitive: true },
			{ name: 'unset-number', description: '', type: 'number' },
		];
		render(<VariablesSettingsModal onChange={vi.fn()} values={{}} variables={noValueVariables} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));

		expect(await screen.findByRole('combobox', { name: 'mode' })).toHaveTextContent('normal');
		expect(screen.getByLabelText('unset-secret')).toHaveValue('');
		expect(screen.getByRole('textbox', { name: 'unset-number' })).toHaveValue('0');
		// `unset-select` has no `values` list at all -- this just asserts that
		// rendering it (with nothing to map over) does not throw.
		expect(screen.getByRole('combobox', { name: 'unset-select' })).toBeInTheDocument();
	});

	it('commits zero when a number field is cleared and blurred with nothing typed back in', async () => {
		const user = userEvent.setup();
		const onChange = vi.fn();
		render(<Harness onChange={onChange} variables={BASE_VARIABLES} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		const field = screen.getByRole('textbox', { name: 'max-players' });
		await user.clear(field);
		await user.tab();

		expect(onChange).toHaveBeenCalledWith('max-players', '0');
	});

	it('renders an open-ended symbol in the range hint when only one bound is set', async () => {
		const user = userEvent.setup();
		const minOnly: ArrowVariable = { name: 'threads', description: '', type: 'number', default: '2', min: 1 };
		const maxOnly: ArrowVariable = { name: 'timeout', description: '', type: 'number', default: '30', max: 120 };
		render(<Harness onChange={vi.fn()} variables={[minOnly, maxOnly]} />);

		await user.click(screen.getByRole('button', { name: 'Configure' }));

		expect(await screen.findByText('1–∞')).toBeInTheDocument();
		expect(screen.getByText('−∞–120')).toBeInTheDocument();
	});
});
