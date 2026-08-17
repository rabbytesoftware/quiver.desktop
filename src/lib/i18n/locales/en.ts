import type { Catalogue, Message } from '../types';

export const en = {
	'locale.name': 'English',

	'app.settings': 'Settings',

	'nav.home': 'Home',
	'nav.remote': 'Remote',
	'nav.settings': 'Settings',
	'nav.back': 'Go back',
	'nav.forward': 'Go forward',

	'search.placeholder': 'Search Arrows or Collections',
	'search.label': 'Search',

	'sidebar.resize': 'Resize sidebar',
	'sidebar.arrows': 'Arrows',
	'sidebar.arrows.loading': 'Loading arrows',
	'sidebar.arrows.empty.title': 'No arrows yet',
	'sidebar.arrows.empty.body': 'Arrows you install show up here.',
	'sidebar.arrows.error.title': 'Can’t reach quiver.core',
	'sidebar.arrows.error.body': 'Your arrows are still installed. This list fills in once the connection is back.',
	'sidebar.arrows.error.action': 'Check connections',
	'arrow.icon.fallback': '{name} icon',

	'settings.title': 'Settings',
	'settings.tab.general': 'General',
	'settings.tab.engine': 'Engine',
	'settings.tab.developer': 'Developer',

	'settings.general.language.title': 'Language',
	'settings.general.language.label': 'Display language',
	'settings.general.language.hint': 'Follows the system language until you pick one here.',
	'settings.general.language.forced':
		'Forced to {language} by VITE_QUIVER_LOCALE for this run. Restart without it to get the picker back.',
	'settings.general.language.system': 'System ({language})',

	'settings.developer.mock.title': 'Mock server',
	'settings.developer.mock.description':
		'Replaces the quiver.core daemon with an in-memory one. Nothing is contacted over the network, and your real library is untouched — mock data lives in its own cache partition.',
	'settings.developer.mock.toggle': 'Use the mock server',
	'settings.developer.mock.forced':
		'Forced on by VITE_QUIVER_MOCK for this run — started by `make dev-mock` or `make dev-web`. Restart without it to get the switch back.',
	'settings.developer.mock.reloads':
		'Turning this on or off reloads the app: which backend is in use is decided once at startup.',
	'settings.developer.mock.scenario': 'Scenario',
	'settings.developer.mock.scenarioLabel': 'Mock scenario',
	'settings.developer.mock.apply': 'Apply',

	'settings.developer.chaos.title': 'Chaos',
	'settings.developer.chaos.description':
		'Applies to the next request. Nothing here is persisted — it all resets when the app restarts.',
	'settings.developer.chaos.inert':
		'Inert while the mock server is off. quiver.core has no equivalent of these, so they cannot be applied to a real daemon.',
	'settings.developer.chaos.latency': 'Latency',
	'settings.developer.chaos.latencyDescription': 'Delay added to every mock response.',
	'settings.developer.chaos.latencyLabel': 'Latency in milliseconds',
	'settings.developer.chaos.errorRate': 'Error rate',
	'settings.developer.chaos.errorRateDescription': 'Chance each request comes back as a daemon-side 500.',
	'settings.developer.chaos.errorRateLabel': 'Error rate percentage',
	'settings.developer.chaos.unreachable': 'Daemon unreachable',
	'settings.developer.chaos.unreachableDescription':
		'Answers every request the way the Rust proxy answers a refused socket — a 502 carrying x-quiver-proxy. This is the only fault that exercises the retry ladder and reaches the Disconnected screen.',
	'settings.developer.chaos.reset': 'Reset chaos',

	'settings.developer.faults.title': 'Fault injection',
	'settings.developer.faults.description':
		"Force one route family to fail, so you can see a single panel's error state without breaking the rest of the app.",
	'settings.developer.faults.rateLabel': '{family} fault rate',
	'settings.developer.faults.reset': 'Reset all faults',
	'settings.developer.faults.arrows': 'Arrow catalog',
	'settings.developer.faults.arrow-detail': 'Arrow detail',
	'settings.developer.faults.search': 'Search',
	'settings.developer.faults.discover': 'Discovery',
	'settings.developer.faults.collections': 'Collections',
	'settings.developer.faults.collection-detail': 'Collection detail',
	'settings.developer.faults.runtime': 'Runtime actions',
	'settings.developer.faults.health': 'Health probe',

	'mock.badge': 'Mock',
	'mock.status': '{scenario} · no daemon is being contacted',
	'mock.turnOff': 'Turn off',
} as const satisfies Catalogue;

export type MessageKey = keyof typeof en;

export type MessageFor<K extends MessageKey> = (typeof en)[K];

export type LocaleCatalogue = Readonly<Record<MessageKey, Message>>;
