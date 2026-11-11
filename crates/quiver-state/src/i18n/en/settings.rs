//! English messages for the `settings.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const TITLE: Key = Key::new("settings.title");
pub const TAB_GENERAL: Key = Key::new("settings.tab.general");
pub const TAB_ENGINE: Key = Key::new("settings.tab.engine");
pub const TAB_DEVELOPER: Key = Key::new("settings.tab.developer");
pub const ROW_RESET: Key = Key::new("settings.row.reset");
pub const GENERAL_APPEARANCE_TITLE: Key = Key::new("settings.general.appearance.title");
pub const GENERAL_THEME_LABEL: Key = Key::new("settings.general.theme.label");
pub const GENERAL_THEME_DESCRIPTION: Key = Key::new("settings.general.theme.description");
pub const GENERAL_THEME_SYSTEM: Key = Key::new("settings.general.theme.system");
pub const GENERAL_THEME_LIGHT: Key = Key::new("settings.general.theme.light");
pub const GENERAL_THEME_DARK: Key = Key::new("settings.general.theme.dark");
pub const GENERAL_SIDEBAR_LABEL: Key = Key::new("settings.general.sidebar.label");
pub const GENERAL_SIDEBAR_DESCRIPTION: Key = Key::new("settings.general.sidebar.description");
pub const GENERAL_SIDEBAR_LEFT: Key = Key::new("settings.general.sidebar.left");
pub const GENERAL_SIDEBAR_RIGHT: Key = Key::new("settings.general.sidebar.right");
pub const GENERAL_LANGUAGE_TITLE: Key = Key::new("settings.general.language.title");
pub const GENERAL_LANGUAGE_LABEL: Key = Key::new("settings.general.language.label");
pub const GENERAL_LANGUAGE_HINT: Key = Key::new("settings.general.language.hint");
pub const GENERAL_LANGUAGE_FORCED: Key = Key::new("settings.general.language.forced");
pub const GENERAL_LANGUAGE_SYSTEM: Key = Key::new("settings.general.language.system");
pub const GENERAL_BUILD_TITLE: Key = Key::new("settings.general.build.title");
pub const GENERAL_BUILD_INDICATOR: Key = Key::new("settings.general.build.indicator");
pub const GENERAL_BUILD_INDICATOR_DESCRIPTION: Key =
    Key::new("settings.general.build.indicatorDescription");
pub const GENERAL_BUILD_CONSOLE: Key = Key::new("settings.general.build.console");
pub const GENERAL_BUILD_CONSOLE_DESCRIPTION: Key =
    Key::new("settings.general.build.consoleDescription");
pub const GENERAL_BUILD_OPEN_CONSOLE: Key = Key::new("settings.general.build.openConsole");
pub const GENERAL_LIBRARY_TITLE: Key = Key::new("settings.general.library.title");
pub const GENERAL_LIBRARY_LABEL: Key = Key::new("settings.general.library.label");
pub const GENERAL_LIBRARY_DESCRIPTION: Key = Key::new("settings.general.library.description");
pub const DEVELOPER_MOCK_TITLE: Key = Key::new("settings.developer.mock.title");
pub const DEVELOPER_MOCK_TOGGLE: Key = Key::new("settings.developer.mock.toggle");
pub const DEVELOPER_MOCK_FORCED: Key = Key::new("settings.developer.mock.forced");
pub const DEVELOPER_MOCK_RELOADS: Key = Key::new("settings.developer.mock.reloads");
pub const DEVELOPER_MOCK_SCENARIO: Key = Key::new("settings.developer.mock.scenario");
pub const DEVELOPER_MOCK_SCENARIO_LABEL: Key = Key::new("settings.developer.mock.scenarioLabel");
pub const DEVELOPER_MOCK_APPLY: Key = Key::new("settings.developer.mock.apply");
pub const DEVELOPER_MOCK_INERT_CONTROLS: Key = Key::new("settings.developer.mock.inertControls");
pub const DEVELOPER_CHAOS_TITLE: Key = Key::new("settings.developer.chaos.title");
pub const DEVELOPER_CHAOS_LATENCY: Key = Key::new("settings.developer.chaos.latency");
pub const DEVELOPER_CHAOS_LATENCY_DESCRIPTION: Key =
    Key::new("settings.developer.chaos.latencyDescription");
pub const DEVELOPER_CHAOS_LATENCY_LABEL: Key = Key::new("settings.developer.chaos.latencyLabel");
pub const DEVELOPER_CHAOS_ERROR_RATE: Key = Key::new("settings.developer.chaos.errorRate");
pub const DEVELOPER_CHAOS_ERROR_RATE_DESCRIPTION: Key =
    Key::new("settings.developer.chaos.errorRateDescription");
pub const DEVELOPER_CHAOS_ERROR_RATE_LABEL: Key =
    Key::new("settings.developer.chaos.errorRateLabel");
pub const DEVELOPER_CHAOS_UNREACHABLE: Key = Key::new("settings.developer.chaos.unreachable");
pub const DEVELOPER_CHAOS_UNREACHABLE_DESCRIPTION: Key =
    Key::new("settings.developer.chaos.unreachableDescription");
pub const DEVELOPER_FAULTS_TITLE: Key = Key::new("settings.developer.faults.title");
pub const DEVELOPER_FAULTS_RATE_LABEL: Key = Key::new("settings.developer.faults.rateLabel");
pub const DEVELOPER_FAULTS_ARROWS: Key = Key::new("settings.developer.faults.arrows");
pub const DEVELOPER_FAULTS_ARROW_DETAIL: Key = Key::new("settings.developer.faults.arrow-detail");
pub const DEVELOPER_FAULTS_SEARCH: Key = Key::new("settings.developer.faults.search");
pub const DEVELOPER_FAULTS_DISCOVER: Key = Key::new("settings.developer.faults.discover");
pub const DEVELOPER_FAULTS_COLLECTIONS: Key = Key::new("settings.developer.faults.collections");
pub const DEVELOPER_FAULTS_COLLECTION_DETAIL: Key =
    Key::new("settings.developer.faults.collection-detail");
pub const DEVELOPER_FAULTS_HOME: Key = Key::new("settings.developer.faults.home");
pub const DEVELOPER_FAULTS_RUNTIME: Key = Key::new("settings.developer.faults.runtime");
pub const DEVELOPER_FAULTS_HEALTH: Key = Key::new("settings.developer.faults.health");
pub const DEVELOPER_FAULTS_CONFIG: Key = Key::new("settings.developer.faults.config");
pub const DEVELOPER_FAULTS_PATH: Key = Key::new("settings.developer.faults.path");
pub const DEVELOPER_FAULTS_CONSOLE: Key = Key::new("settings.developer.faults.console");
pub const DEVELOPER_NATIVE_FORCED: Key = Key::new("settings.developer.native.forced");
pub const DEVELOPER_NATIVE_OFF: Key = Key::new("settings.developer.native.off");
pub const DEVELOPER_NATIVE_CHAOS_NOTE: Key = Key::new("settings.developer.native.chaosNote");
pub const ENGINE_LOADING: Key = Key::new("settings.engine.loading");
pub const ENGINE_RESTART: Key = Key::new("settings.engine.restart");
pub const ENGINE_CORRECTED: Key = Key::new("settings.engine.corrected");
pub const ENGINE_RETRY: Key = Key::new("settings.engine.retry");
pub const ENGINE_CHANNEL_RETRY: Key = Key::new("settings.engine.channelRetry");
pub const ENGINE_PORTS_TITLE: Key = Key::new("settings.engine.ports.title");
pub const ENGINE_PORTS_LABEL: Key = Key::new("settings.engine.ports.label");
pub const ENGINE_PORTS_DESCRIPTION: Key = Key::new("settings.engine.ports.description");
pub const ENGINE_PORTS_LOWEST: Key = Key::new("settings.engine.ports.lowest");
pub const ENGINE_PORTS_HIGHEST: Key = Key::new("settings.engine.ports.highest");
pub const ENGINE_PORTS_FIELD_REJECTED: Key = Key::new("settings.engine.ports.fieldRejected");
pub const ENGINE_LOGS_TITLE: Key = Key::new("settings.engine.logs.title");
pub const ENGINE_LOGS_DISK: Key = Key::new("settings.engine.logs.disk");
pub const ENGINE_LOGS_DISK_DESCRIPTION: Key = Key::new("settings.engine.logs.diskDescription");
pub const ENGINE_LOGS_LEVEL: Key = Key::new("settings.engine.logs.level");
pub const ENGINE_LOGS_LEVEL_DESCRIPTION: Key = Key::new("settings.engine.logs.levelDescription");
pub const ENGINE_LOGS_LEVEL_DEBUG: Key = Key::new("settings.engine.logs.level.debug");
pub const ENGINE_LOGS_LEVEL_INFO: Key = Key::new("settings.engine.logs.level.info");
pub const ENGINE_LOGS_LEVEL_WARN: Key = Key::new("settings.engine.logs.level.warn");
pub const ENGINE_LOGS_LEVEL_ERROR: Key = Key::new("settings.engine.logs.level.error");
pub const ENGINE_SELF_UPDATE_TITLE: Key = Key::new("settings.engine.selfUpdate.title");
pub const ENGINE_SELF_UPDATE_CHANNEL: Key = Key::new("settings.engine.selfUpdate.channel");
pub const ENGINE_SELF_UPDATE_CHANNEL_DESCRIPTION: Key =
    Key::new("settings.engine.selfUpdate.channelDescription");
pub const ENGINE_SELF_UPDATE_LOADING: Key = Key::new("settings.engine.selfUpdate.loading");
pub const ENGINE_SELF_UPDATE_UNSET: Key = Key::new("settings.engine.selfUpdate.unset");
pub const ENGINE_SELF_UPDATE_INSTALLED: Key = Key::new("settings.engine.selfUpdate.installed");
pub const ENGINE_SELF_UPDATE_AVAILABLE: Key = Key::new("settings.engine.selfUpdate.available");
pub const ENGINE_SELF_UPDATE_CURRENT: Key = Key::new("settings.engine.selfUpdate.current");
pub const ENGINE_SELF_UPDATE_UNKNOWN: Key = Key::new("settings.engine.selfUpdate.unknown");
pub const ENGINE_SELF_UPDATE_CHECK: Key = Key::new("settings.engine.selfUpdate.check");
pub const ENGINE_SELF_UPDATE_RESTARTS: Key = Key::new("settings.engine.selfUpdate.restarts");
pub const ENGINE_SELF_UPDATE_UNAVAILABLE: Key = Key::new("settings.engine.selfUpdate.unavailable");
pub const ENGINE_AUTO_REGISTER_TITLE: Key = Key::new("settings.engine.autoRegister.title");
pub const ENGINE_AUTO_REGISTER_LABEL: Key = Key::new("settings.engine.autoRegister.label");
pub const ENGINE_AUTO_REGISTER_DESCRIPTION: Key =
    Key::new("settings.engine.autoRegister.description");
pub const ENGINE_AUTO_REGISTER_PENDING: Key = Key::new("settings.engine.autoRegister.pending");
pub const CORE_UPDATES_TITLE: Key = Key::new("settings.core.updates.title");
pub const CORE_VERSION_CHECK_LABEL: Key = Key::new("settings.core.versionCheck.label");
pub const CORE_VERSION_CHECK_DESCRIPTION: Key = Key::new("settings.core.versionCheck.description");
pub const CORE_MANIFESTS_TITLE: Key = Key::new("settings.core.manifests.title");
pub const CORE_FETCH_TIMEOUT_LABEL: Key = Key::new("settings.core.fetchTimeout.label");
pub const CORE_FETCH_TIMEOUT_DESCRIPTION: Key = Key::new("settings.core.fetchTimeout.description");
pub const CORE_AUTO_RETRY_LABEL: Key = Key::new("settings.core.autoRetry.label");
pub const CORE_AUTO_RETRY_DESCRIPTION: Key = Key::new("settings.core.autoRetry.description");
pub const CORE_RETRIES_LABEL: Key = Key::new("settings.core.retries.label");
pub const CORE_RETRIES_DESCRIPTION: Key = Key::new("settings.core.retries.description");
pub const CORE_SEARCH_TITLE: Key = Key::new("settings.core.search.title");
pub const CORE_PER_SOURCE_LABEL: Key = Key::new("settings.core.perSource.label");
pub const CORE_PER_SOURCE_DESCRIPTION: Key = Key::new("settings.core.perSource.description");
pub const CORE_SOURCE_TIMEOUT_LABEL: Key = Key::new("settings.core.sourceTimeout.label");
pub const CORE_SOURCE_TIMEOUT_DESCRIPTION: Key =
    Key::new("settings.core.sourceTimeout.description");
pub const CORE_MIN_STARS_LABEL: Key = Key::new("settings.core.minStars.label");
pub const CORE_MIN_STARS_DESCRIPTION: Key = Key::new("settings.core.minStars.description");
pub const CORE_RECOMMENDATIONS_TITLE: Key = Key::new("settings.core.recommendations.title");
pub const CORE_RECOMMENDATIONS_LABEL: Key = Key::new("settings.core.recommendations.label");
pub const CORE_RECOMMENDATIONS_DESCRIPTION: Key =
    Key::new("settings.core.recommendations.description");
pub const CORE_REFRESH_LABEL: Key = Key::new("settings.core.refresh.label");
pub const CORE_REFRESH_DESCRIPTION: Key = Key::new("settings.core.refresh.description");
pub const CORE_NETWORK_TITLE: Key = Key::new("settings.core.network.title");
pub const CORE_PORT_FORWARDING_LABEL: Key = Key::new("settings.core.portForwarding.label");
pub const CORE_PORT_FORWARDING_DESCRIPTION: Key =
    Key::new("settings.core.portForwarding.description");
pub const CORE_PENDING_RESTART: Key = Key::new("settings.core.pendingRestart");
pub const ENGINE_PATH_TITLE: Key = Key::new("settings.engine.path.title");
pub const ENGINE_PATH_LABEL: Key = Key::new("settings.engine.path.label");
pub const ENGINE_PATH_LOADING: Key = Key::new("settings.engine.path.loading");
pub const ENGINE_PATH_SETUP: Key = Key::new("settings.engine.path.setup");
pub const ENGINE_PATH_FILES: Key = Key::new("settings.engine.path.files");
pub const ENGINE_PATH_STATE_READY: Key = Key::new("settings.engine.path.state.ready");
pub const ENGINE_PATH_STATE_RESTART: Key = Key::new("settings.engine.path.state.restart");
pub const ENGINE_PATH_DESCRIPTION_SETUP: Key = Key::new("settings.engine.path.description.setup");
pub const ENGINE_PATH_DESCRIPTION_RESTART: Key =
    Key::new("settings.engine.path.description.restart");
pub const ENGINE_PATH_DESCRIPTION_READY: Key = Key::new("settings.engine.path.description.ready");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (TITLE, Message::Text("Settings")),
    (TAB_GENERAL, Message::Text("General")),
    (TAB_ENGINE, Message::Text("Engine")),
    (TAB_DEVELOPER, Message::Text("Developer")),
    (ROW_RESET, Message::Text("Reset {setting}")),
    (GENERAL_APPEARANCE_TITLE, Message::Text("Appearance")),
    (GENERAL_THEME_LABEL, Message::Text("Theme")),
    (GENERAL_THEME_DESCRIPTION, Message::Text("Use light, dark, or follow system preference")),
    (GENERAL_THEME_SYSTEM, Message::Text("System")),
    (GENERAL_THEME_LIGHT, Message::Text("Light")),
    (GENERAL_THEME_DARK, Message::Text("Dark")),
    (GENERAL_SIDEBAR_LABEL, Message::Text("Sidebar side")),
    (GENERAL_SIDEBAR_DESCRIPTION, Message::Text("Which side the sidebar appears on")),
    (GENERAL_SIDEBAR_LEFT, Message::Text("Left")),
    (GENERAL_SIDEBAR_RIGHT, Message::Text("Right")),
    (GENERAL_LANGUAGE_TITLE, Message::Text("Language")),
    (GENERAL_LANGUAGE_LABEL, Message::Text("Display language")),
    (GENERAL_LANGUAGE_HINT, Message::Text("Language for Quiver’s interface")),
    (GENERAL_LANGUAGE_FORCED, Message::Text("Forced to {language} by VITE_QUIVER_LOCALE for this run. Restart without it to get the picker back.")),
    (GENERAL_LANGUAGE_SYSTEM, Message::Text("System ({language})")),
    (GENERAL_BUILD_TITLE, Message::Text("Builds")),
    (GENERAL_BUILD_INDICATOR, Message::Text("Build indicator")),
    (GENERAL_BUILD_INDICATOR_DESCRIPTION, Message::Text("Shows which quiver.core and quiver.desktop builds are running, at the top of the sidebar. Off by default on stable releases.")),
    (GENERAL_BUILD_CONSOLE, Message::Text("Daemon console")),
    (GENERAL_BUILD_CONSOLE_DESCRIPTION, Message::Text("The connected daemon’s log, and a prompt for its commands. Esc closes it.")),
    (GENERAL_BUILD_OPEN_CONSOLE, Message::Text("Open console")),
    (GENERAL_LIBRARY_TITLE, Message::Text("Library")),
    (GENERAL_LIBRARY_LABEL, Message::Text("Show Quiver's own components")),
    (GENERAL_LIBRARY_DESCRIPTION, Message::Text("Include quiver.core and quiver.desktop's own rows in Home, Library, Search, and the sidebar")),
    (DEVELOPER_MOCK_TITLE, Message::Text("Mock server")),
    (DEVELOPER_MOCK_TOGGLE, Message::Text("Use the mock server")),
    (DEVELOPER_MOCK_FORCED, Message::Text("Forced on by VITE_QUIVER_MOCK for this run — started by `make dev-mock` or `make dev-web`. Restart without it to get the switch back.")),
    (DEVELOPER_MOCK_RELOADS, Message::Text("Turning this on or off reloads the app: which backend is in use is decided once at startup.")),
    (DEVELOPER_MOCK_SCENARIO, Message::Text("Scenario")),
    (DEVELOPER_MOCK_SCENARIO_LABEL, Message::Text("Mock scenario")),
    (DEVELOPER_MOCK_APPLY, Message::Text("Apply")),
    (DEVELOPER_MOCK_INERT_CONTROLS, Message::Text("Chaos and fault injection do nothing while the mock server is off — turn it on above to try them.")),
    (DEVELOPER_CHAOS_TITLE, Message::Text("Chaos")),
    (DEVELOPER_CHAOS_LATENCY, Message::Text("Latency")),
    (DEVELOPER_CHAOS_LATENCY_DESCRIPTION, Message::Text("Delay added to every mock response.")),
    (DEVELOPER_CHAOS_LATENCY_LABEL, Message::Text("Latency in milliseconds")),
    (DEVELOPER_CHAOS_ERROR_RATE, Message::Text("Error rate")),
    (DEVELOPER_CHAOS_ERROR_RATE_DESCRIPTION, Message::Text("Chance each request comes back as a daemon-side 500.")),
    (DEVELOPER_CHAOS_ERROR_RATE_LABEL, Message::Text("Error rate percentage")),
    (DEVELOPER_CHAOS_UNREACHABLE, Message::Text("Daemon unreachable")),
    (DEVELOPER_CHAOS_UNREACHABLE_DESCRIPTION, Message::Text("Answers every request the way the Rust proxy answers a refused socket — a 502 carrying x-quiver-proxy. This is the only fault that exercises the retry ladder and reaches the Disconnected screen.")),
    (DEVELOPER_FAULTS_TITLE, Message::Text("Fault injection")),
    (DEVELOPER_FAULTS_RATE_LABEL, Message::Text("{family} fault rate")),
    (DEVELOPER_FAULTS_ARROWS, Message::Text("Arrow catalog")),
    (DEVELOPER_FAULTS_ARROW_DETAIL, Message::Text("Arrow detail")),
    (DEVELOPER_FAULTS_SEARCH, Message::Text("Search")),
    (DEVELOPER_FAULTS_DISCOVER, Message::Text("Discovery")),
    (DEVELOPER_FAULTS_COLLECTIONS, Message::Text("Collections")),
    (DEVELOPER_FAULTS_COLLECTION_DETAIL, Message::Text("Collection detail")),
    (DEVELOPER_FAULTS_HOME, Message::Text("Home")),
    (DEVELOPER_FAULTS_RUNTIME, Message::Text("Runtime actions")),
    (DEVELOPER_FAULTS_HEALTH, Message::Text("Health probe")),
    (DEVELOPER_FAULTS_CONFIG, Message::Text("Daemon config")),
    (DEVELOPER_FAULTS_PATH, Message::Text("Command line PATH")),
    (DEVELOPER_FAULTS_CONSOLE, Message::Text("Console")),
    (DEVELOPER_NATIVE_FORCED, Message::Text("Forced on for this run — Quiver was started with --demo, so every screen shows seeded data and no daemon is contacted. Restart without it to use a real daemon.")),
    (DEVELOPER_NATIVE_OFF, Message::Text("Quiver is talking to a real daemon. Start it with --demo to use the seeded mock data instead; which backend is in use is decided once at startup.")),
    (DEVELOPER_NATIVE_CHAOS_NOTE, Message::Text("The demo data is served from memory, so latency, error rate and the fault rates are only recorded for this session: nothing is delayed or refused. Daemon unreachable does take effect, it drops the connection to Disconnected.")),
    (ENGINE_LOADING, Message::Text("Loading engine settings")),
    (ENGINE_RESTART, Message::Text("These settings take effect the next time quiver.core restarts.")),
    (ENGINE_CORRECTED, Message::Text("The daemon could not use these settings and fell back to their defaults: {settings}.")),
    (ENGINE_RETRY, Message::Text("Try again")),
    (ENGINE_CHANNEL_RETRY, Message::Text("Retry")),
    (ENGINE_PORTS_TITLE, Message::Text("Ports")),
    (ENGINE_PORTS_LABEL, Message::Text("Ports for servers")),
    (ENGINE_PORTS_DESCRIPTION, Message::Text("Range Quiver may use for arrows that serve. Match your router’s forwarding rules.")),
    (ENGINE_PORTS_LOWEST, Message::Text("Lowest port")),
    (ENGINE_PORTS_HIGHEST, Message::Text("Highest port")),
    (ENGINE_PORTS_FIELD_REJECTED, Message::Text("{field}: {message}")),
    (ENGINE_LOGS_TITLE, Message::Text("Logs")),
    (ENGINE_LOGS_DISK, Message::Text("Write logs to disk")),
    (ENGINE_LOGS_DISK_DESCRIPTION, Message::Text("Keeps a rotating file alongside the console output")),
    (ENGINE_LOGS_LEVEL, Message::Text("Level")),
    (ENGINE_LOGS_LEVEL_DESCRIPTION, Message::Text("How much detail the daemon writes")),
    (ENGINE_LOGS_LEVEL_DEBUG, Message::Text("Debug")),
    (ENGINE_LOGS_LEVEL_INFO, Message::Text("Info")),
    (ENGINE_LOGS_LEVEL_WARN, Message::Text("Warn")),
    (ENGINE_LOGS_LEVEL_ERROR, Message::Text("Error")),
    (ENGINE_SELF_UPDATE_TITLE, Message::Text("Updates")),
    (ENGINE_SELF_UPDATE_CHANNEL, Message::Text("Update channel")),
    (ENGINE_SELF_UPDATE_CHANNEL_DESCRIPTION, Message::Text("Which release channel quiver.core tracks for its own updates")),
    (ENGINE_SELF_UPDATE_LOADING, Message::Text("Loading channels…")),
    (ENGINE_SELF_UPDATE_UNSET, Message::Text("Not set")),
    (ENGINE_SELF_UPDATE_INSTALLED, Message::Text("Installed version")),
    (ENGINE_SELF_UPDATE_AVAILABLE, Message::Text("Available version")),
    (ENGINE_SELF_UPDATE_CURRENT, Message::Text("Up to date")),
    (ENGINE_SELF_UPDATE_UNKNOWN, Message::Text("Unknown")),
    (ENGINE_SELF_UPDATE_CHECK, Message::Text("Check for updates")),
    (ENGINE_SELF_UPDATE_RESTARTS, Message::Text("quiver.core restarts to finish updating. The connection drops for a moment and comes back on its own.")),
    (ENGINE_SELF_UPDATE_UNAVAILABLE, Message::Text("Couldn't load quiver.core's channels")),
    (ENGINE_AUTO_REGISTER_TITLE, Message::Text("Arrows")),
    (ENGINE_AUTO_REGISTER_LABEL, Message::Text("Auto-register arrows from repositories without an ARROW.md")),
    (ENGINE_AUTO_REGISTER_DESCRIPTION, Message::Text("Builds an arrow from a repository’s release downloads when it ships no manifest.")),
    (ENGINE_AUTO_REGISTER_PENDING, Message::Text("Restart quiver.core to apply this change.")),
    (CORE_UPDATES_TITLE, Message::Text("Updates")),
    (CORE_VERSION_CHECK_LABEL, Message::Text("Check installed arrows for updates every")),
    (CORE_VERSION_CHECK_DESCRIPTION, Message::Text("Soon after quiver.core starts, then this often, so an arrow shows it is outdated without being opened. 0s turns it off.")),
    (CORE_MANIFESTS_TITLE, Message::Text("Manifests")),
    (CORE_FETCH_TIMEOUT_LABEL, Message::Text("Download timeout")),
    (CORE_FETCH_TIMEOUT_DESCRIPTION, Message::Text("How long quiver.core waits for an ARROW.md or a release file before giving up.")),
    (CORE_AUTO_RETRY_LABEL, Message::Text("Retry manifests of followed collections")),
    (CORE_AUTO_RETRY_DESCRIPTION, Message::Text("When following a collection, a member whose manifest fails to load is tried again.")),
    (CORE_RETRIES_LABEL, Message::Text("Retries")),
    (CORE_RETRIES_DESCRIPTION, Message::Text("How many more times a member is tried. 0 means a single attempt.")),
    (CORE_SEARCH_TITLE, Message::Text("Search")),
    (CORE_PER_SOURCE_LABEL, Message::Text("Results per source")),
    (CORE_PER_SOURCE_DESCRIPTION, Message::Text("The most results one search takes from each source.")),
    (CORE_SOURCE_TIMEOUT_LABEL, Message::Text("Source timeout")),
    (CORE_SOURCE_TIMEOUT_DESCRIPTION, Message::Text("How long a search waits for each source before counting it as unavailable.")),
    (CORE_MIN_STARS_LABEL, Message::Text("Stars needed without a manifest")),
    (CORE_MIN_STARS_DESCRIPTION, Message::Text("A repository with no ARROW.md shows up in search only with at least this many stars.")),
    (CORE_RECOMMENDATIONS_TITLE, Message::Text("Recommendations")),
    (CORE_RECOMMENDATIONS_LABEL, Message::Text("Recommend arrows on Home")),
    (CORE_RECOMMENDATIONS_DESCRIPTION, Message::Text("Fills Home's shelves from the network.")),
    (CORE_REFRESH_LABEL, Message::Text("Refresh recommendations every")),
    (CORE_REFRESH_DESCRIPTION, Message::Text("How often the shelves are read again from the network.")),
    (CORE_NETWORK_TITLE, Message::Text("Network")),
    (CORE_PORT_FORWARDING_LABEL, Message::Text("Open ports on the router")),
    (CORE_PORT_FORWARDING_DESCRIPTION, Message::Text("Asks the router (UPnP / NAT-PMP) to forward the ports arrows serve on. Off, the ports are still given out, just not opened on the router.")),
    (CORE_PENDING_RESTART, Message::Text("Saved. Restart quiver.core to apply it.")),
    (ENGINE_PATH_TITLE, Message::Text("Command line")),
    (ENGINE_PATH_LABEL, Message::Text("Use Quiver commands in a terminal")),
    (ENGINE_PATH_LOADING, Message::Text("Checking your PATH…")),
    (ENGINE_PATH_SETUP, Message::Text("Set up PATH")),
    (ENGINE_PATH_FILES, Message::Text("Configuration: {files}.")),
    (ENGINE_PATH_STATE_READY, Message::Text("Ready")),
    (ENGINE_PATH_STATE_RESTART, Message::Text("Configured")),
    (ENGINE_PATH_DESCRIPTION_SETUP, Message::Text("Add Quiver’s command directory ({dir}) to your PATH so the commands of installed arrows work in a terminal.")),
    (ENGINE_PATH_DESCRIPTION_RESTART, Message::Text("Quiver’s command directory is set up on your PATH. Open a new terminal, or restart your shell, for it to take effect.")),
    (ENGINE_PATH_DESCRIPTION_READY, Message::Text("Quiver’s command directory is on your PATH, so the commands of installed arrows work in a terminal.")),
];
