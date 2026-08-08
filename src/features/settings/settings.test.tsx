import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useMockStore } from '@/lib/mock/store';

import { Section, SettingRow } from './components/section';
import { visibleTabs } from './components/settings-dialog';
import { UNLOCK_CLICKS, VersionUnlock } from './components/version-unlock';
import { rowMatchesQuery, useSettingsUI } from './store';

beforeEach(() => {
	useSettingsUI.setState({ open: false, tab: 'connections', query: '' });
	useMockStore.setState({ devUnlocked: false });
});

describe('row search', () => {
	it('matches on the description, not only the label', () => {
		expect(rowMatchesQuery('retry', 'Daemon unreachable', 'exercises the retry ladder')).toBe(true);
	});

	it('matches everything when the query is blank or whitespace', () => {
		expect(rowMatchesQuery('   ', 'Anything')).toBe(true);
	});

	it('is case-insensitive', () => {
		expect(rowMatchesQuery('LATENCY', 'Latency')).toBe(true);
	});
});

describe('a section whose rows all filter out', () => {
	function Fixture() {
		return (
			<Section title="Chaos" description="knobs">
				<SettingRow label="Latency" />
				<SettingRow label="Error rate" />
			</Section>
		);
	}

	it('keeps its heading while at least one row survives', () => {
		useSettingsUI.setState({ query: 'latency' });
		render(<Fixture />);
		expect(screen.getByText('Chaos').closest('section')).not.toHaveAttribute('hidden');
		expect(screen.getByText('Latency')).toBeInTheDocument();
		expect(screen.queryByText('Error rate')).not.toBeInTheDocument();
	});

	// An orphaned heading with nothing under it reads as "there are results here
	// that failed to render" rather than "there are none".
	it('hides its heading once every row is gone', () => {
		useSettingsUI.setState({ query: 'zzzzz' });
		render(<Fixture />);
		expect(screen.getByText('Chaos').closest('section')).toHaveAttribute('hidden');
	});
});

describe('the developer tab', () => {
	it('is present in dev without any gesture', () => {
		// vitest runs with import.meta.env.DEV true, which is the dev case.
		expect(visibleTabs(false).map((t) => t.id)).toEqual(['general', 'connections', 'developer']);
	});

	it('is listed once the unlock flag is set', () => {
		expect(visibleTabs(true).map((t) => t.id)).toContain('developer');
	});
});

describe('the version unlock', () => {
	it('opens after exactly seven taps, and not six', async () => {
		const user = userEvent.setup();
		render(<VersionUnlock />);
		const version = screen.getByRole('button', { name: /Quiver version/ });

		for (let i = 0; i < UNLOCK_CLICKS - 1; i++) await user.click(version);
		expect(useMockStore.getState().devUnlocked).toBe(false);

		await user.click(version);
		expect(useMockStore.getState().devUnlocked).toBe(true);
	});

	// Silent early on: a countdown from the first click is how it would get
	// found by accident, which is the one thing it exists to prevent.
	it('says nothing about the countdown until you are most of the way there', async () => {
		const user = userEvent.setup();
		render(<VersionUnlock />);
		const version = screen.getByRole('button', { name: /Quiver version/ });

		await user.click(version);
		await user.click(version);
		expect(screen.queryByText(/more tap/)).not.toBeInTheDocument();

		await user.click(version);
		expect(screen.getByText(/more taps…/)).toBeInTheDocument();
	});
});

describe('the settings dialog store', () => {
	it('clears the search when it closes, so it does not reopen filtered', () => {
		useSettingsUI.getState().openSettings('developer');
		useSettingsUI.getState().setQuery('fault');
		useSettingsUI.getState().closeSettings();

		expect(useSettingsUI.getState().query).toBe('');
		// The tab, unlike the query, is remembered.
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
