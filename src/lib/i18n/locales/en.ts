import type { Catalogue, Message } from '../types';

/**
 * The source locale. Every other catalogue is defined against this one.
 *
 * FLAT DOTTED KEYS, not nested objects. Nesting reads better in a diff and is
 * worse at everything else that matters here: `keyof typeof en` gives the key
 * union for free where a nested tree needs a recursive path type that TypeScript
 * will eventually refuse to expand, `satisfies Record<MessageKey, Message>` is
 * what makes an incomplete translation a compile error, and the guard in
 * locales.test.ts becomes a `for` over `Object.keys` rather than a tree walk.
 *
 * `as const` is load-bearing twice over: it is what gives each value a literal
 * string type, which is what lets `PlaceholdersIn` read the `{name}` holes out
 * of it — drop it and every message widens to `string`, `ParamsFor` collapses
 * to `undefined`, and `t` stops asking for interpolation values entirely,
 * silently. `satisfies Catalogue` then checks the shape without widening it,
 * which `: Catalogue` would.
 *
 * TO ADD A LOCALE:
 *   1. copy this file to `./<tag>.ts`, translate the values, and type it
 *      `satisfies LocaleCatalogue` (exported below) rather than `Catalogue` —
 *      that is what makes a missing or misspelled key fail `tsc`;
 *   2. add it to `CATALOGUES` in `../catalogue.ts`. That is the whole
 *      registration: `Locale`, `LOCALES` and the Settings picker are all
 *      derived from that object;
 *   3. give it its own `locale.name`, written in that language — a French
 *      speaker scanning the picker is looking for "Français", not "French".
 *
 * Nothing here is a translation of anything a daemon says. Host names, arrow
 * descriptions and error messages come off the wire in whatever language
 * quiver.core produced them in, and are passed through untouched.
 */
export const en = {
	// The language's own name, in that language. Read out of each catalogue by
	// the Settings picker, so adding a locale never means editing a second list.
	'locale.name': 'English',

	'app.settings': 'Settings',

	// The rail's three destinations, then the two history glyphs. Back and
	// forward render as bare icons, so these are the whole accessible name — no
	// label, and a screen reader announces them as "button".
	'nav.home': 'Home',
	'nav.remote': 'Remote',
	'nav.settings': 'Settings',
	'nav.back': 'Go back',
	'nav.forward': 'Go forward',

	// Names what is searched rather than the act of searching. The field sits in
	// the rail now, directly above the library it filters, so "Search" alone left
	// the one question a new user actually has — what is in here? — unanswered by
	// the only text on screen.
	//
	// `search.label` stays the short form: it is the accessible NAME, and a
	// screen reader announcing the placeholder as well would read the long
	// version twice.
	'search.placeholder': 'Search Arrows or Collections',
	'search.label': 'Search',

	'sidebar.resize': 'Resize sidebar',
	'sidebar.arrows': 'Arrows',
	// An arrow that ships no icon falls back to a lettered tile. Two such tiles
	// are indistinguishable to a screen reader without the name in the label.
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
	// The one plural in the app today, and the reason `t` takes `count` rather
	// than letting call sites pick the form with a ternary: a ternary is correct
	// in English and wrong in every language with a third category.
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
	// One per FAULT_KEYS entry in `@/lib/mock/store`. The key suffixes ARE those
	// slugs, so `t(`settings.developer.faults.${key}`)` type-checks: a fault key
	// added without a label fails to compile at the call site.
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

/** Dotted key of every message in the app. A `t` call with anything else fails `tsc`. */
export type MessageKey = keyof typeof en;

/** The literal message behind a key, which is what `ParamsFor` reads. */
export type MessageFor<K extends MessageKey> = (typeof en)[K];

/**
 * What a non-source locale has to be. Required (not `Partial`) keys are the
 * point: an untranslated key is a build failure, not a runtime fallback that
 * nobody notices until a screenshot comes back half in English.
 */
export type LocaleCatalogue = Readonly<Record<MessageKey, Message>>;
