import { useState } from 'react';

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import { useLocale, useTranslation } from './react';
import { useLocaleStore, type LocalePreference } from './store';

beforeEach(() => {
	useLocaleStore.setState({ preference: 'system', detected: 'en' });
});

describe('useTranslation', () => {
	it('renders a message and its interpolation', () => {
		function Fixture() {
			const { t } = useTranslation();
			return (
				<>
					<span>{t('settings.title')}</span>
					<span>{t('arrow.icon.fallback', { name: 'Sample' })}</span>
				</>
			);
		}

		render(<Fixture />);
		expect(screen.getByText('Settings')).toBeInTheDocument();
		expect(screen.getByText('Sample icon')).toBeInTheDocument();
	});

	it('binds the Intl formatters to the same locale', () => {
		function Fixture() {
			const { formatNumber, formatPercent } = useTranslation();
			return <span>{`${formatNumber(1234.5)} ${formatPercent(0.4)}`}</span>;
		}

		render(<Fixture />);
		expect(screen.getByText('1,234.5 40%')).toBeInTheDocument();
	});

	it('hands back the same translator across a render that did not change the locale', async () => {
		const seen: unknown[] = [];

		function Fixture() {
			const translator = useTranslation();
			const [n, setN] = useState(0);
			seen.push(translator);
			return (
				<button type="button" onClick={() => setN(n + 1)}>
					{n}
				</button>
			);
		}

		const user = userEvent.setup();
		render(<Fixture />);
		await user.click(screen.getByRole('button'));

		expect(seen.length).toBeGreaterThan(1);
		expect(new Set(seen).size).toBe(1);
	});
});

describe('useLocale', () => {
	it('reports the locale in force and follows a change to the preference', async () => {
		function Fixture() {
			const locale = useLocale();
			const setPreference = useLocaleStore((s) => s.setPreference);
			return (
				<button type="button" onClick={() => setPreference('en' as LocalePreference)}>
					{locale}
				</button>
			);
		}

		const user = userEvent.setup();
		render(<Fixture />);
		expect(screen.getByRole('button')).toHaveTextContent('en');

		await user.click(screen.getByRole('button'));
		expect(useLocaleStore.getState().preference).toBe('en');
		expect(screen.getByRole('button')).toHaveTextContent('en');
	});
});
