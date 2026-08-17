import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createPortal } from 'react-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { create } from 'zustand';

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

	// Mirrors the Engine Ports row: `onReset` is async (a patch to the
	// daemon) and the control it targets is keyed on a value owned by a
	// store, so it remounts under a new key once the reset resolves. A fix
	// that re-focuses the control captured *before* the await would be
	// focusing a node React has since thrown away — this only proves
	// anything if the query genuinely happens after the await settles.
	//
	// Uses a real zustand store (not `useState`) for the keyed value: like
	// `useEngineStore`, its subscription goes through
	// `useSyncExternalStore`, whose commit is scheduled ahead of a plain
	// `Promise`/`setTimeout` continuation — a `useState` update racing the
	// same `setTimeout` would land *after* the focus call and pass for the
	// wrong reason.
	it('re-queries after an async reset, so focus still lands on a control remounted under a new key', async () => {
		const user = userEvent.setup();
		const useKeyStore = create<{ key: number; bump: () => void }>((set) => ({
			key: 0,
			bump: () => set((s) => ({ key: s.key + 1 })),
		}));

		function AsyncResetRow() {
			const key = useKeyStore((s) => s.key);
			return (
				<SettingRow
					label="Theme"
					canReset
					onReset={() =>
						new Promise<void>((resolve) => {
							setTimeout(() => {
								useKeyStore.getState().bump();
								resolve();
							}, 0);
						})
					}
				>
					<input key={key} aria-label="Theme value" defaultValue="dark" />
				</SettingRow>
			);
		}

		render(<AsyncResetRow />);
		await user.click(screen.getByRole('button', { name: 'Reset Theme' }));

		await waitFor(() => expect(screen.getByRole('textbox', { name: 'Theme value' })).toHaveFocus());
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

	// A drag-select ends with `mouseup` inside whatever element the pointer
	// is over — most usefully the description — which still bubbles a
	// `click` up to the row. Without a check for an active selection, that
	// `click` would activate the control (`.select()`-ing a number input or
	// `.click()`-ing a switch), destroying the very selection the user just
	// made.
	it('leaves an active text selection alone instead of activating the control', async () => {
		const user = userEvent.setup();
		const onClick = vi.fn();
		const selection = vi.spyOn(window, 'getSelection').mockReturnValue({
			toString: () => 'selected text',
		} as Selection);

		render(
			<SettingRow label="Mock server" description="a description">
				<button type="button" onClick={onClick}>
					toggle
				</button>
			</SettingRow>
		);
		await user.click(screen.getByText('a description'));
		expect(onClick).not.toHaveBeenCalled();

		selection.mockRestore();
	});

	// Base UI's `Select` renders its option list into `document.body` via a
	// portal, but a portalled node is still a CHILD of `SettingRow` in the
	// React tree, and React replays a click through the component tree, not
	// the DOM tree. So a click on an option arrives at `activate()` looking
	// exactly like a click on the row's own dead space.
	//
	// The option itself carries `role="option"`, which — like Base UI's real
	// options — is not in `PASSTHROUGH`, so the passthrough check alone
	// cannot save it. Only a DOM-containment check (is the click's target
	// actually inside this row's own subtree?) can tell the two apart, since
	// the portal node lives under `document.body`, not under the row.
	it('ignores a click on a portalled descendant even though it is a child in the React tree', async () => {
		const user = userEvent.setup();
		const onClick = vi.fn();

		function PortalledOption() {
			return createPortal(
				<div role="option" aria-selected={false} data-testid="portalled">
					option
				</div>,
				document.body
			);
		}

		render(
			<SettingRow label="Mock server">
				<button type="button" onClick={onClick}>
					toggle
				</button>
				<PortalledOption />
			</SettingRow>
		);

		await user.click(screen.getByTestId('portalled'));

		expect(onClick).not.toHaveBeenCalled();
		expect(screen.getByRole('button', { name: 'toggle' })).not.toHaveFocus();
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
