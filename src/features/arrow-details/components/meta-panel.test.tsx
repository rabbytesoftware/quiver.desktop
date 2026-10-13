import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import type { ArrowCredit, ArrowPort, ArrowRequirement } from '@/domain/arrow';
import type { DependencyRow } from '@/features/arrow-details/lib/dependency-rows';

import { MetaPanel } from './meta-panel';

/** `MetaPanel`'s dependency rows are real `Link`s -- only those tests need a router around them. */
function renderWithRouter(ui: React.ReactElement) {
	const rootRoute = createRootRoute({ component: () => ui });
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});
	const router = createRouter({
		routeTree: rootRoute.addChildren([arrowRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});
	return render(<RouterProvider router={router} />);
}

const REQUIREMENT: ArrowRequirement = { cpu_cores: 4, disk_gb: 20, memory_gb: 8 };

const PORTS: ArrowPort[] = [
	{ default: 25565, name: 'game', protocol: 'tcp', required: true },
	{ default: 25575, name: 'rcon', protocol: 'udp', required: false },
];

const MAINTAINERS: ArrowCredit[] = [{ email: 'ada@example.com', name: 'Ada Lovelace', url: 'https://example.com/ada' }];

const CREDITS: ArrowCredit[] = [{ name: 'Grace Hopper' }];

const DEPENDS_ON: DependencyRow[] = [
	{
		namespace: 'github.com/rabbyte/nats@v2.10.24',
		name: 'NATS',
		icon: null,
		ref: 'v2.10.24',
		state: 'ready',
		userInstalled: true,
	},
];

const REQUIRED_BY: DependencyRow[] = [
	{
		namespace: 'github.com/rabbyte/discord@v1.2.0',
		name: 'github.com/rabbyte/discord',
		icon: null,
		ref: 'v1.2.0',
		state: 'absent',
		userInstalled: false,
	},
];

describe('MetaPanel', () => {
	it('renders the Details title', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="" />);
		expect(screen.getByText('Details')).toBeInTheDocument();
	});

	it('omits every section, including headers, when there is nothing to show', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="" />);
		expect(screen.queryByText('Requirements')).not.toBeInTheDocument();
		expect(screen.queryByText('Network')).not.toBeInTheDocument();
		expect(screen.queryByText('Maintainers')).not.toBeInTheDocument();
		expect(screen.queryByText('Credits')).not.toBeInTheDocument();
		expect(screen.queryByText('Links')).not.toBeInTheDocument();
	});

	it('renders the Requirements section only when a requirement is given', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} requirement={REQUIREMENT} url="" />);

		expect(screen.getByText('Requirements')).toBeInTheDocument();
		expect(screen.getByText('CPU')).toBeInTheDocument();
		expect(screen.getByText('4 cores')).toBeInTheDocument();
		expect(screen.getByText('Memory')).toBeInTheDocument();
		expect(screen.getByText('8 GB')).toBeInTheDocument();
		expect(screen.getByText('Disk')).toBeInTheDocument();
		expect(screen.getByText('20 GB')).toBeInTheDocument();
	});

	it('does not render Requirements when no requirement is given', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="" />);
		expect(screen.queryByText('CPU')).not.toBeInTheDocument();
	});

	it('renders one row per port, with its protocol badge, port number, and a required flag only when required', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={PORTS} url="" />);

		expect(screen.getByText('Network')).toBeInTheDocument();
		expect(screen.getByText('game')).toBeInTheDocument();
		expect(screen.getByText('tcp')).toBeInTheDocument();
		expect(screen.getByText('25565')).toBeInTheDocument();
		expect(screen.getByText('required')).toBeInTheDocument();

		expect(screen.getByText('rcon')).toBeInTheDocument();
		expect(screen.getByText('udp')).toBeInTheDocument();
		expect(screen.getByText('25575')).toBeInTheDocument();
	});

	it('omits Network when netbridge is empty', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="" />);
		expect(screen.queryByText('Network')).not.toBeInTheDocument();
	});

	it('links a maintainer name when a url is present', () => {
		render(<MetaPanel credits={[]} maintainers={MAINTAINERS} netbridge={[]} url="" />);

		expect(screen.getByText('Maintainers')).toBeInTheDocument();
		const link = screen.getByRole('link', { name: 'Ada Lovelace' });
		expect(link).toHaveAttribute('href', 'https://example.com/ada');
		expect(link).toHaveAttribute('target', '_blank');
		expect(link).toHaveAttribute('rel', 'noreferrer');
	});

	it('renders a credit as plain text, not a link, when it has no url', () => {
		render(<MetaPanel credits={CREDITS} maintainers={[]} netbridge={[]} url="" />);

		expect(screen.getByText('Credits')).toBeInTheDocument();
		expect(screen.getByText('Grace Hopper')).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: 'Grace Hopper' })).not.toBeInTheDocument();
	});

	it('omits Maintainers and Credits when both lists are empty', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="" />);
		expect(screen.queryByText('Maintainers')).not.toBeInTheDocument();
		expect(screen.queryByText('Credits')).not.toBeInTheDocument();
	});

	it('renders the Links section with a wrapping url link when url is non-empty', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="https://example.com/repo" />);

		expect(screen.getByText('Links')).toBeInTheDocument();
		const link = screen.getByRole('link', { name: 'https://example.com/repo' });
		expect(link).toHaveAttribute('href', 'https://example.com/repo');
		expect(link).toHaveAttribute('target', '_blank');
	});

	it('omits Links when url is empty', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="" />);
		expect(screen.queryByText('Links')).not.toBeInTheDocument();
	});

	it('renders every section together when everything is present', () => {
		render(
			<MetaPanel
				credits={CREDITS}
				maintainers={MAINTAINERS}
				netbridge={PORTS}
				requirement={REQUIREMENT}
				url="https://example.com/repo"
			/>
		);

		expect(screen.getByText('Requirements')).toBeInTheDocument();
		expect(screen.getByText('Network')).toBeInTheDocument();
		expect(screen.getByText('Maintainers')).toBeInTheDocument();
		expect(screen.getByText('Credits')).toBeInTheDocument();
		expect(screen.getByText('Links')).toBeInTheDocument();
	});

	it('omits Dependencies and Required by when neither is given', () => {
		render(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} url="" />);
		expect(screen.queryByText('Dependencies')).not.toBeInTheDocument();
		expect(screen.queryByText('Required by')).not.toBeInTheDocument();
	});

	it('renders one row per dependency, as a link to that arrow, with a status badge from the catalog', async () => {
		renderWithRouter(<MetaPanel credits={[]} dependsOn={DEPENDS_ON} maintainers={[]} netbridge={[]} url="" />);

		expect(await screen.findByText('Dependencies')).toBeInTheDocument();
		const link = screen.getByRole('link', { name: /NATS/ });
		expect(link).toHaveAttribute('href', '/arrow/github.com/rabbyte/nats%40v2.10.24');
		expect(screen.getByText('Ready')).toBeInTheDocument();
	});

	it('renders a dependent not in the local catalog as its bare namespace, with a Discovered badge', async () => {
		renderWithRouter(<MetaPanel credits={[]} maintainers={[]} netbridge={[]} requiredBy={REQUIRED_BY} url="" />);

		expect(await screen.findByText('Required by')).toBeInTheDocument();
		expect(screen.getByText('github.com/rabbyte/discord')).toBeInTheDocument();
		expect(screen.getByText('Not in library')).toBeInTheDocument();
	});
});
