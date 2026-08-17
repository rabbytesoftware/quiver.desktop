import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useMockStore } from '@/lib/mock/store';

import { Section, SettingRow } from './components/section';
import { useSettingsUI } from './store';
import { visibleTabs } from './tabs';

beforeEach(() => {
	useSettingsUI.setState({ tab: 'general' });
});

describe('Section', () => {
	it('renders its title and children', () => {
		render(
			<Section title="Appearance">
				<SettingRow label="Theme" />
			</Section>
		);
		expect(screen.getByText('Appearance')).toBeInTheDocument();
		expect(screen.getByText('Theme')).toBeInTheDocument();
	});
});

describe('SettingRow reset', () => {
	it('has no reset control when onReset is not given', () => {
		render(<SettingRow label="Theme" />);
		expect(screen.queryByRole('button', { name: /reset/i })).not.toBeInTheDocument();
	});

	it('reserves the reset control while it cannot be used, so the label never shifts', () => {
		render(<SettingRow label="Theme" onReset={() => {}} canReset={false} />);
		const reset = screen.getByRole('button', { name: 'Reset Theme' });
		expect(reset).toBeDisabled();
		expect(reset).toHaveClass('invisible');
	});

	it('calls onReset when it can be used', async () => {
		const user = userEvent.setup();
		const onReset = vi.fn();
		render(<SettingRow label="Theme" onReset={onReset} canReset />);
		await user.click(screen.getByRole('button', { name: 'Reset Theme' }));
		expect(onReset).toHaveBeenCalledOnce();
	});

	// `canReset` flips false the instant a reset succeeds, applying both
	// `disabled` and `invisible` to the button — either alone drops it from
	// the focus chain. Without moving focus somewhere on purpose, a keyboard
	// user who activates the reset is dumped onto <body>.
	it('moves focus to the row control instead of dropping it on activation', async () => {
		const user = userEvent.setup();
		render(
			<SettingRow label="Theme" onReset={() => {}} canReset>
				<input aria-label="Theme value" defaultValue="dark" />
			</SettingRow>
		);
		await user.click(screen.getByRole('button', { name: 'Reset Theme' }));
		expect(screen.getByRole('textbox', { name: 'Theme value' })).toHaveFocus();
		expect(document.body).not.toHaveFocus();
	});
});

describe('SettingRow click delegation', () => {
	it('activates the row control when the row itself is clicked', async () => {
		const user = userEvent.setup();
		const onClick = vi.fn();
		render(
			<SettingRow label="Mock server" description="a description">
				<button type="button" onClick={onClick}>
					toggle
				</button>
			</SettingRow>
		);
		await user.click(screen.getByText('a description'));
		expect(onClick).toHaveBeenCalledOnce();
	});

	it('does not double-fire when the control itself is clicked', async () => {
		const user = userEvent.setup();
		const onClick = vi.fn();
		render(
			<SettingRow label="Mock server">
				<button type="button" onClick={onClick}>
					toggle
				</button>
			</SettingRow>
		);
		await user.click(screen.getByRole('button', { name: 'toggle' }));
		expect(onClick).toHaveBeenCalledOnce();
	});
});

describe('visibleTabs', () => {
	it('lists general and engine, with developer only in a dev build', () => {
		expect(visibleTabs().map((t) => t.id)).toEqual(['general', 'engine', 'developer']);
	});

	it('drops developer in a release build', () => {
		vi.stubEnv('DEV', false);
		expect(visibleTabs().map((t) => t.id)).toEqual(['general', 'engine']);
		vi.unstubAllEnvs();
	});
});

describe('the settings store', () => {
	it('remembers the tab across a visit', () => {
		useSettingsUI.getState().setTab('developer');
		expect(useSettingsUI.getState().tab).toBe('developer');
	});
});

describe('applyAndReload', () => {
	it('persists the choice BEFORE reloading, or the reload would read the old value', () => {
		const reload = vi.fn();
		Object.defineProperty(window, 'location', {
			value: { ...window.location, reload },
			writable: true,
		});

		useMockStore.getState().applyAndReload({ enabled: true, scenario: 'extreme' });

		expect(useMockStore.getState().enabled).toBe(true);
		expect(useMockStore.getState().scenario).toBe('extreme');
		expect(reload).toHaveBeenCalled();

		const persisted = JSON.parse(localStorage.getItem('quiver.mock') ?? '{}') as {
			state?: { enabled?: boolean; scenario?: string };
		};
		expect(persisted.state?.enabled).toBe(true);
		expect(persisted.state?.scenario).toBe('extreme');
	});
});
