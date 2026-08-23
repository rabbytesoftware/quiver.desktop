import type { Catalogue, Message } from '../types';

export const en = {
	'locale.name': 'English',

	'app.settings': 'Settings',

	'nav.home': 'Home',
	'nav.remote': 'Remote',
	'nav.settings': 'Settings',
	'nav.back': 'Go back',
	'nav.forward': 'Go forward',

	'search.placeholder': 'Search Arrows',
	'search.label': 'Search',

	'search.results.count': {
		one: '{count} result',
		other: '{count} results',
	},
	'search.results.searching': 'Searching the network…',
	'search.results.passFailed': 'The network search did not finish.',
	'search.results.refused': '{host} refused',
	'search.results.retry': 'Retry in {seconds}s',
	'search.results.inspect': 'Inspect',
	'search.results.unreachable': 'Quiver is not running',
	'search.results.idle': 'Type to search.',
	'search.results.emptyEverywhere': 'Nothing matched, and every host answered.',
	'search.results.emptyWithRefusal': 'Nothing matched on the hosts that answered.',

	'search.card.stars': {
		one: '{count} star',
		other: '{count} stars',
	},
	'search.provenance.installed': 'installed',
	'search.provenance.dependency': 'dependency',
	'search.provenance.collection': 'collection',
	'search.provenance.seen': 'seen',

	'search.shelf.vault': 'In your vault',
	'search.shelf.network': 'From the network',
	'search.shelf.soFar': '{count} so far',

	'search.results.hosts': {
		one: '{count} host',
		other: '{count} hosts',
	},
	'search.results.refusedCount': {
		one: '{count} refused',
		other: '{count} refused',
	},

	'search.sort.label': 'Sort results',
	'search.sort.relevance': 'Relevance',
	'search.sort.name': 'Name',
	'search.sort.stars': 'Stars',

	'search.narrow.label': 'Narrow results',
	'search.narrow.facet': {
		one: '{value}, {count} result',
		other: '{value}, {count} results',
	},
	'search.narrow.clear': 'Clear',
	'search.narrow.empty': 'Nothing matches that combination.',

	'search.inspector.title': 'This search',
	'search.inspector.description': 'What the pass did, and what it was allowed to do.',
	'search.inspector.pass': 'The pass',
	'search.inspector.query': 'Query',
	'search.inspector.job': 'Job ID',
	'search.inspector.counts': 'Found / verified / skipped',
	'search.inspector.expires': 'Expires at',
	'search.inspector.hosts': 'Hosts',
	'search.inspector.settings': 'Settings',
	'search.inspector.asked': '{count} asked',
	'search.inspector.running': 'The job carries no providers until the pass ends.',
	'search.inspector.close': 'Close',

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
	'settings.search.placeholder': 'Search settings',
	'settings.search.label': 'Search settings',
	'settings.tab.general': 'General',
	'settings.tab.connections': 'Connections',
	'settings.tab.developer': 'Developer',

	'settings.general.language.title': 'Language',
	'settings.general.language.description':
		'Applies to Quiver’s own interface. Anything a daemon sends — host names, arrow descriptions, errors — arrives in whatever language it was written in and is shown as-is.',
	'settings.general.language.label': 'Display language',
	'settings.general.language.hint': 'Follows the system language until you pick one here.',
	'settings.general.language.forced':
		'Forced to {language} by VITE_QUIVER_LOCALE for this run. Restart without it to get the picker back.',
	'settings.general.language.system': 'System ({language})',
	'settings.general.formats.title': 'Formats',
	'settings.general.formats.description':
		'Dates and numbers follow the display language, not a separate setting — a preview, so the effect of the row above is visible before anything else in the app uses it.',
	'settings.general.formats.date': 'Date',
	'settings.general.formats.number': 'Number',

	'settings.connections.hosts.title': 'Hosts',
	'settings.connections.hosts.description':
		'Every quiver.core daemon this app can talk to. Switching keeps each host’s cached library separate.',
	'settings.connections.hosts.mocked':
		'The mock server is on, so this list is fabricated and cannot be changed. Turn it off in Developer to manage real hosts.',
	'settings.connections.host.fabricated': 'Fabricated — there is no daemon behind this',
	'settings.connections.host.bundled': 'Bundled daemon',
	'settings.connections.host.active': 'Active',
	'settings.connections.host.switch': 'Switch',
	'settings.connections.host.remove': 'Remove',
	'settings.connections.add.title': 'Add a host',
	'settings.connections.add.description':
		'A remote quiver.core daemon. The token is stored in the OS keychain, never in the app’s own storage.',
	'settings.connections.add.name': 'Name',
	'settings.connections.add.namePlaceholder': 'Basement box',
	'settings.connections.add.nameLabel': 'Host name',
	'settings.connections.add.url': 'URL',
	'settings.connections.add.urlLabel': 'Host URL',
	'settings.connections.add.token': 'Token',
	'settings.connections.add.tokenLabel': 'Host token',
	'settings.connections.add.submit': 'Add host',

	'settings.version.label': 'Quiver version {version}',
	'settings.version.text': 'Quiver {version}',
	'settings.version.remaining': {
		one: '{count} more tap…',
		other: '{count} more taps…',
	},
	'settings.version.unlocked': 'Developer tab unlocked.',

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
	'settings.developer.faults.config': 'Configuration',

	'mock.badge': 'Mock',
	'mock.status': '{scenario} · no daemon is being contacted',
	'mock.turnOff': 'Turn off',
} as const satisfies Catalogue;

export type MessageKey = keyof typeof en;

export type MessageFor<K extends MessageKey> = (typeof en)[K];

export type LocaleCatalogue = Readonly<Record<MessageKey, Message>>;
