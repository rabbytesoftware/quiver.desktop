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
	'sidebar.arrows.error.action': 'Check Engine settings',
	'arrow.icon.fallback': '{name} icon',

	'settings.title': 'Settings',
	'settings.tab.general': 'General',
	'settings.tab.engine': 'Engine',
	'settings.tab.developer': 'Developer',
	'settings.row.reset': 'Reset {setting}',

	'settings.general.appearance.title': 'Appearance',
	'settings.general.theme.label': 'Theme',
	'settings.general.theme.description': 'Use light, dark, or follow system preference',
	'settings.general.theme.system': 'System',
	'settings.general.theme.light': 'Light',
	'settings.general.theme.dark': 'Dark',
	'settings.general.sidebar.label': 'Sidebar side',
	'settings.general.sidebar.description': 'Which side the sidebar appears on',
	'settings.general.sidebar.left': 'Left',
	'settings.general.sidebar.right': 'Right',

	'settings.general.language.title': 'Language',
	'settings.general.language.label': 'Display language',
	'settings.general.language.hint': 'Language for Quiver’s interface',
	'settings.general.language.forced':
		'Forced to {language} by VITE_QUIVER_LOCALE for this run. Restart without it to get the picker back.',
	'settings.general.language.system': 'System ({language})',

	'settings.developer.mock.title': 'Mock server',
	'settings.developer.mock.toggle': 'Use the mock server',
	'settings.developer.mock.forced':
		'Forced on by VITE_QUIVER_MOCK for this run — started by `make dev-mock` or `make dev-web`. Restart without it to get the switch back.',
	'settings.developer.mock.reloads':
		'Turning this on or off reloads the app: which backend is in use is decided once at startup.',
	'settings.developer.mock.scenario': 'Scenario',
	'settings.developer.mock.scenarioLabel': 'Mock scenario',
	'settings.developer.mock.apply': 'Apply',
	'settings.developer.mock.inertControls':
		'Chaos and fault injection do nothing while the mock server is off — turn it on above to try them.',

	'settings.developer.chaos.title': 'Chaos',
	'settings.developer.chaos.latency': 'Latency',
	'settings.developer.chaos.latencyDescription': 'Delay added to every mock response.',
	'settings.developer.chaos.latencyLabel': 'Latency in milliseconds',
	'settings.developer.chaos.errorRate': 'Error rate',
	'settings.developer.chaos.errorRateDescription': 'Chance each request comes back as a daemon-side 500.',
	'settings.developer.chaos.errorRateLabel': 'Error rate percentage',
	'settings.developer.chaos.unreachable': 'Daemon unreachable',
	'settings.developer.chaos.unreachableDescription':
		'Answers every request the way the Rust proxy answers a refused socket — a 502 carrying x-quiver-proxy. This is the only fault that exercises the retry ladder and reaches the Disconnected screen.',

	'settings.developer.faults.title': 'Fault injection',
	'settings.developer.faults.rateLabel': '{family} fault rate',
	'settings.developer.faults.arrows': 'Arrow catalog',
	'settings.developer.faults.arrow-detail': 'Arrow detail',
	'settings.developer.faults.search': 'Search',
	'settings.developer.faults.discover': 'Discovery',
	'settings.developer.faults.collections': 'Collections',
	'settings.developer.faults.collection-detail': 'Collection detail',
	'settings.developer.faults.runtime': 'Runtime actions',
	'settings.developer.faults.health': 'Health probe',
	'settings.developer.faults.config': 'Daemon config',

	'settings.engine.loading': 'Loading engine settings',
	'settings.engine.restart': 'These settings take effect the next time quiver.core restarts.',
	'settings.engine.corrected': 'The daemon could not use these settings and fell back to their defaults: {settings}.',
	'settings.engine.retry': 'Try again',
	'settings.engine.ports.title': 'Ports',
	'settings.engine.ports.label': 'Ports for servers',
	'settings.engine.ports.description':
		'Range Quiver may use for arrows that serve. Match your router’s forwarding rules.',
	'settings.engine.ports.lowest': 'Lowest port',
	'settings.engine.ports.highest': 'Highest port',
	'settings.engine.ports.fieldRejected': '{field}: {message}',
	'settings.engine.logs.title': 'Logs',
	'settings.engine.logs.disk': 'Write logs to disk',
	'settings.engine.logs.diskDescription': 'Keeps a rotating file alongside the console output',
	'settings.engine.logs.level': 'Level',
	'settings.engine.logs.levelDescription': 'How much detail the daemon writes',
	'settings.engine.logs.level.debug': 'Debug',
	'settings.engine.logs.level.info': 'Info',
	'settings.engine.logs.level.warn': 'Warn',
	'settings.engine.logs.level.error': 'Error',

	'mock.badge': 'Mock',
	'mock.status': '{scenario} · no daemon is being contacted',
	'mock.turnOff': 'Turn off',

	'arrow.error': "Couldn't load this arrow.",
	'arrow.loading': 'Loading…',
	'arrow.tab.overview': 'Overview',
	'arrow.tab.activity': 'Activity',
	'arrow.tab.methods': 'Methods',
	'arrow.tab.settings': 'Settings',
	'arrow.tab.group': '{a} / {b}',

	'arrow.state.discovered': 'Not in library',
	'arrow.state.absent': 'Not installed',
	'arrow.state.installing': 'Installing…',
	'arrow.state.ready': 'Ready',
	'arrow.state.outdated': 'Update available',
	'arrow.state.updating': 'Updating…',
	'arrow.state.running': 'Running',
	'arrow.state.stopping': 'Stopping…',
	'arrow.state.draining': 'Draining…',
	'arrow.state.detached': 'Detached',
	'arrow.state.uninstalling': 'Uninstalling…',
	'arrow.state.removed': 'Removed',

	'arrow.action.addToLibrary': 'Add to Library',
	'arrow.action.install': 'Install',
	'arrow.action.installing': 'Installing…',
	'arrow.action.removeFromLibrary': 'Remove from Library',
	'arrow.action.start': 'Start',
	'arrow.action.uninstall': 'Uninstall',
	'arrow.action.uninstalling': 'Uninstalling…',
	'arrow.action.update': 'Update',
	'arrow.action.updating': 'Updating…',
	'arrow.action.stop': 'Stop',
	'arrow.action.stopping': 'Stopping…',
	'arrow.action.draining': 'Draining…',
	'arrow.action.restart': 'Restart',
	'arrow.action.reinstall': 'Reinstall',
	'arrow.action.info': 'What this does',

	'arrow.problem.label': 'Issue',
	'arrow.problem.detachedNote':
		'Quiver lost track of this process. It may still be running outside Quiver’s supervision — use Stop if you want to kill it.',
	'arrow.problem.failedNote': 'The last run did not finish successfully.',

	'arrow.step.inspect': 'Inspect step definition',
	'arrow.step.modal.title': 'Raw step definition',
	'arrow.step.type.run': 'run',
	'arrow.step.type.fetch': 'fetch',
	'arrow.step.type.signal': 'signal',
	'arrow.step.type.dependencies': 'deps',

	'arrow.preview.subtitle': 'What runs when you choose this',
	'arrow.preview.uses': 'Uses',
	'arrow.preview.configure': 'Configure',

	'arrow.settings.title': 'Settings',
	'arrow.settings.subtitle': 'Used to configure how this arrow runs. Changes apply the next time a method is called.',
	'arrow.settings.done': 'Done',
	'arrow.settings.summary': {
		one: '{count} setting',
		other: '{count} settings',
	},
	'arrow.settings.summarySensitive': '{summary} · {sensitive} sensitive',
	'arrow.settings.reveal': 'Reveal',
	'arrow.settings.hide': 'Hide',

	'arrow.details.title': 'Details',
	'arrow.details.requirements': 'Requirements',
	'arrow.details.requirements.cpu': 'CPU',
	'arrow.details.requirements.cpu.value': '{count} cores',
	'arrow.details.requirements.memory': 'Memory',
	'arrow.details.requirements.memory.value': '{count} GB',
	'arrow.details.requirements.disk': 'Disk',
	'arrow.details.requirements.disk.value': '{count} GB',
	'arrow.details.network': 'Network',
	'arrow.details.network.required': 'required',
	'arrow.details.maintainers': 'Maintainers',
	'arrow.details.credits': 'Credits',
	'arrow.details.links': 'Links',
	'arrow.details.dependencies': 'Dependencies',
	'arrow.details.requiredBy': 'Required by',

	'arrow.readme.taskDone': 'Done',
	'arrow.readme.taskTodo': 'To do',

	'arrow.activity.title': 'Activity',
	'arrow.activity.empty': 'No activity yet.',
	'arrow.activity.emptyNotInstalled': 'Add this arrow to your library to install and run it.',
	'arrow.activity.outcome.success': 'Succeeded',
	'arrow.activity.outcome.failed': 'Failed',
	'arrow.activity.outcome.cancelled': 'Cancelled',
	'arrow.methods.count': {
		one: '{count} method',
		other: '{count} methods',
	},

	'arrow.version.label': 'Version',

	'home.recents': 'Recents',
	'home.library': 'Library',
	'home.collections': 'Collections',
	'home.viewAllArrows': {
		one: 'View {count} arrow',
		other: 'View all {count} arrows',
	},
	'home.viewAllCollections': {
		one: 'View {count} collection',
		other: 'View all {count} collections',
	},
	'home.empty.title': 'Nothing here yet',
	'home.empty.description': 'Installed arrows and the collections you follow will show up here.',
	'home.empty.cta': 'Search for arrows',

	'library.subtitle': {
		one: '{count} installed arrow',
		other: '{count} installed arrows',
	},
	'library.sort.name': 'Sort: Name',

	'collections.subtitle': '{count} followed',
	'collections.arrowCount': {
		one: '{count} arrow',
		other: '{count} arrows',
	},

	'remote.title': 'Remote Control',
	'remote.subtitle': 'Manage your saved connections.',
	'remote.addButton': 'Add remote',

	'remote.local.subtitle': 'This device',

	'remote.status.starting': 'Connecting…',
	'remote.status.ready': 'Connected',
	'remote.status.disconnected': 'Disconnected',
	'remote.status.disconnectedReason':
		"Couldn't reach the daemon. Check the URL, that Quiver is running there, and that the pairing hasn't expired.",

	'remote.action.retry': 'Retry',
	'remote.action.switchToLocal': 'Switch to Local',

	'remote.menu.more': 'More actions',
	'remote.menu.switch': 'Switch to this connection',
	'remote.menu.rename': 'Rename',
	'remote.menu.remove': 'Remove',

	'remote.empty.title': 'No remote daemons yet',
	'remote.empty.description': 'Add one to connect to Quiver running elsewhere on your network.',

	'remote.add.title': 'Add a remote connection',
	'remote.add.description': 'Point this app at a Quiver daemon running on another machine.',
	'remote.add.descriptionPairing': 'Almost there — enter the pairing code from that machine.',
	'remote.add.name.label': 'Name',
	'remote.add.name.placeholder': 'Home Lab',
	'remote.add.url.label': 'URL',
	'remote.add.url.placeholder': 'http://10.0.1.8:7420',
	'remote.add.code.label': 'Pairing code',
	'remote.add.code.hint':
		'Run "quiver auth generate" on that machine, then enter the code shown. It expires in 5 minutes and works once.',
	'remote.add.cancel': 'Cancel',
	'remote.add.back': 'Back',
	'remote.add.continue': 'Continue',
	'remote.add.checking': 'Checking…',
	'remote.add.healthError': "Couldn't reach that address. Check the URL and try again.",
	'remote.add.submit': 'Add connection',
	'remote.add.submitting': 'Adding…',

	'remote.rename.title': 'Rename connection',
	'remote.rename.label': 'Name',
	'remote.rename.cancel': 'Cancel',
	'remote.rename.submit': 'Save',

	'remote.remove.title': 'Remove {name}?',
	'remote.remove.description': "You'll need a new pairing code to reconnect later.",
	'remote.remove.activeWarning': 'This is your current connection — removing it switches you back to Local.',
	'remote.remove.cancel': 'Cancel',
	'remote.remove.submit': 'Remove',

	'remote.toast.connected': 'Connected to {name}',
	'remote.toast.disconnected': "Couldn't reach {name}",
	'remote.toast.added': 'Added {name}',
	'remote.toast.renamed': 'Renamed to {name}',
	'remote.toast.removed': 'Removed {name}',
	'remote.toast.connectFailed': "Couldn't switch to {name}",
	'remote.toast.addFailed': "Couldn't add {name}",
	'remote.toast.renameFailed': "Couldn't rename to {name}",
	'remote.toast.removeFailed': "Couldn't remove {name}",

	'remote.switcher.label': 'Switch connection',

	'remote.command.placeholder': 'Switch connection…',
	'remote.command.groupLabel': 'Connections',
	'remote.command.current': 'Current',
	'remote.command.empty': 'No connections match "{query}".',
	'remote.command.navigate': 'Navigate',
	'remote.command.select': 'Select',
	'remote.command.close': 'Close',
} as const satisfies Catalogue;

export type MessageKey = keyof typeof en;

export type MessageFor<K extends MessageKey> = (typeof en)[K];

export type LocaleCatalogue = Readonly<Record<MessageKey, Message>>;
