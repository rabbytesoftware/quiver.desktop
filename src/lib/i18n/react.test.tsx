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
	it('renders a message, its interpolation and its plural form', () => {
		function Fixture() {
			const { t } = useTranslation();
			return (
				<>
					<span>{t('settings.title')}</span>
					<span>{t('settings.version.text', { version: '0.1.0' })}</span>
					<span>{t('settings.version.remaining', { count: 2 })}</span>
				</>
			);
		}

		render(<Fixture />);
		expect(screen.getByText('Settings')).toBeInTheDocument();
		expect(screen.getByText('Quiver 0.1.0')).toBeInTheDocument();
		expect(screen.getByText('2 more taps…')).toBeInTheDocument();
	});

	it('binds the Intl formatters to the same locale', () => {
		function Fixture() {
			const { formatNumber, formatPercent } = useTranslation();
			return <span>{`${formatNumber(1234.5)} ${formatPercent(0.4)}`}</span>;
		}

		render(<Fixture />);
		expect(screen.getByText('1,234.5 40%')).toBeInTheDocument();
	});

	// The regression this guards is invisible at the call site and expensive
	// three components down: an unmemoised translator has a new identity every
	// render, which defeats every `useMemo`, `useCallback` and `React.memo`
	// that lists `t` as a dependency. Nobody would trace that back to i18n.
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

		// One catalogue ships, so the resolved locale cannot move yet. What this
		// pins is that the component is SUBSCRIBED — a selector that threw, or
		// one returning a fresh object and looping, would fail here rather than
		// on the day a second catalogue lands.
		await user.click(screen.getByRole('button'));
		expect(useLocaleStore.getState().preference).toBe('en');
		expect(screen.getByRole('button')).toHaveTextContent('en');
	});
});
