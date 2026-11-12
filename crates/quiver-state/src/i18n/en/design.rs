//! Strings the new design adds (progress control, tiles, methods modal, glance panel).

use crate::i18n::message::{Key, Message};

pub const STEP_OF: Key = Key::new("arrow.progress.stepOf");
pub const STARTING: Key = Key::new("arrow.progress.starting");
pub const STEPS: Key = Key::new("arrow.progress.steps");
pub const STATUS_NEEDS_ATTENTION: Key = Key::new("arrow.status.needsAttention");
pub const STATUS_UPDATE_AVAILABLE: Key = Key::new("arrow.status.updateAvailable");
pub const METHODS_INSPECT_ONLY: Key = Key::new("arrow.methods.inspectOnly");
pub const METHODS_INSPECT_NOTE: Key = Key::new("arrow.methods.inspectNote");
pub const METHODS_LIFECYCLE: Key = Key::new("arrow.methods.lifecycle");
pub const METHODS_CUSTOM: Key = Key::new("arrow.methods.custom");
pub const SETTINGS_OPEN: Key = Key::new("arrow.settings.open");
pub const PLATFORM_UNAVAILABLE_HERE: Key = Key::new("arrow.platform.unavailableHere");
pub const PLATFORM_RUNS_ON: Key = Key::new("arrow.platform.runsOn");
pub const GLANCE_TITLE: Key = Key::new("arrow.glance.title");
pub const GLANCE_METHODS: Key = Key::new("arrow.glance.methods");
pub const GLANCE_SETTINGS: Key = Key::new("arrow.glance.settings");
pub const PLAIN_RUN: Key = Key::new("arrow.stepPlain.run");
pub const PLAIN_FETCH: Key = Key::new("arrow.stepPlain.fetch");
pub const PLAIN_EXTRACT: Key = Key::new("arrow.stepPlain.extract");
pub const PLAIN_PORTABLE: Key = Key::new("arrow.stepPlain.portable");
pub const PLAIN_SIGNAL: Key = Key::new("arrow.stepPlain.signal");
pub const PLAIN_DEPENDENCIES: Key = Key::new("arrow.stepPlain.dependencies");
pub const PLAIN_EXPOSE: Key = Key::new("arrow.stepPlain.expose");
pub const PLAIN_UNEXPOSE: Key = Key::new("arrow.stepPlain.unexpose");
pub const PLAIN_OTHER: Key = Key::new("arrow.stepPlain.other");
pub const TILE_RUNNING: Key = Key::new("tile.badge.running");
pub const TILE_NEEDS_ATTENTION: Key = Key::new("tile.badge.needsAttention");
pub const TILE_UPDATING: Key = Key::new("tile.badge.updating");
pub const TILE_STOPPING: Key = Key::new("tile.badge.stopping");
pub const TILE_UNINSTALLING: Key = Key::new("tile.badge.uninstalling");
pub const TILE_INSTALLING: Key = Key::new("tile.badge.installing");
pub const TILE_UPDATE: Key = Key::new("tile.badge.update");
pub const TILE_INSTALLED: Key = Key::new("tile.badge.installed");
pub const TILE_NOT_INSTALLED: Key = Key::new("tile.badge.notInstalled");
pub const TILE_NOT_IN_LIBRARY: Key = Key::new("tile.badge.notInLibrary");
pub const VERB_INSTALLING: Key = Key::new("arrow.progress.verb.installing");
pub const VERB_UPDATING: Key = Key::new("arrow.progress.verb.updating");
pub const VERB_UNINSTALLING: Key = Key::new("arrow.progress.verb.uninstalling");
pub const VERB_STOPPING: Key = Key::new("arrow.progress.verb.stopping");
pub const VERB_DRAINING: Key = Key::new("arrow.progress.verb.draining");
pub const VERB_STARTING: Key = Key::new("arrow.progress.verb.starting");
pub const SEARCH_SOURCES: Key = Key::new("search.sources");
pub const SEARCH_UNAVAILABLE: Key = Key::new("search.sourcesUnavailable");
pub const LIBRARY_FILTER_ALL: Key = Key::new("library.filter.all");
pub const LIBRARY_FILTER_RUNNING: Key = Key::new("library.filter.running");
pub const LIBRARY_FILTER_UPDATES: Key = Key::new("library.filter.updates");
pub const POPULAR_TITLE: Key = Key::new("popular.title");
pub const POPULAR_LOADED: Key = Key::new("popular.loaded");
pub const POPULAR_ALL: Key = Key::new("popular.all");
pub const COLLECTION_ARROWS: Key = Key::new("collection.arrowCount");
pub const COLLECTION_ALL_INSTALLED: Key = Key::new("collection.allInstalled");
pub const COLLECTION_UNRESOLVED_TITLE: Key = Key::new("collection.unresolved.title");
pub const COLLECTION_UNRESOLVED_RETRY: Key = Key::new("collection.unresolved.retry");
pub const COLLECTION_FOLLOWING: Key = Key::new("collection.following");
pub const COLLECTION_FOLLOW: Key = Key::new("collection.follow");
pub const REMOTE_THIS_DEVICE: Key = Key::new("remote.thisDevice");
pub const REMOTE_REMOTE: Key = Key::new("remote.remoteLabel");
pub const REMOTE_NOT_CONNECTED: Key = Key::new("remote.notConnected");
pub const APP_MENU_SETTINGS: Key = Key::new("arrowApp.menu.settings");
pub const APP_MENU_METHODS: Key = Key::new("arrowApp.menu.methods");
pub const APP_MENU_OPEN_PAGE: Key = Key::new("arrowApp.menu.openPage");
pub const APP_MENU_UNINSTALL: Key = Key::new("arrowApp.menu.uninstall");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (STEP_OF, Message::Text("{title} · {index} of {total}")),
    (STARTING, Message::Text("Starting…")),
    (STEPS, Message::Text("Steps")),
    (STATUS_NEEDS_ATTENTION, Message::Text("Needs attention")),
    (STATUS_UPDATE_AVAILABLE, Message::Text("Update available")),
    (
        METHODS_INSPECT_ONLY,
        Message::Text("Methods · inspect only"),
    ),
    (
        METHODS_INSPECT_NOTE,
        Message::Text("What this arrow can do. Inspect only: methods are not run from here."),
    ),
    (METHODS_LIFECYCLE, Message::Text("Lifecycle")),
    (METHODS_CUSTOM, Message::Text("Custom")),
    (SETTINGS_OPEN, Message::Text("Settings")),
    (
        PLATFORM_UNAVAILABLE_HERE,
        Message::Text("Not available for this {os}"),
    ),
    (PLATFORM_RUNS_ON, Message::Text("Runs on {platforms}")),
    (GLANCE_TITLE, Message::Text("At a glance")),
    (GLANCE_METHODS, Message::Text("{count} methods · {names}")),
    (GLANCE_SETTINGS, Message::Text("{count} settings")),
    (PLAIN_RUN, Message::Text("Runs a command on this machine.")),
    (PLAIN_FETCH, Message::Text("Downloads a file.")),
    (
        PLAIN_EXTRACT,
        Message::Text("Unpacks the downloaded archive."),
    ),
    (
        PLAIN_PORTABLE,
        Message::Text("Prepares the arrow's portable files."),
    ),
    (
        PLAIN_SIGNAL,
        Message::Text("Sends a signal to the running process."),
    ),
    (
        PLAIN_DEPENDENCIES,
        Message::Text("Makes sure the arrows this one needs are installed."),
    ),
    (
        PLAIN_EXPOSE,
        Message::Text("Opens the arrow's network ports."),
    ),
    (
        PLAIN_UNEXPOSE,
        Message::Text("Closes the arrow's network ports."),
    ),
    (PLAIN_OTHER, Message::Text("Runs a step of this method.")),
    (TILE_RUNNING, Message::Text("Running")),
    (TILE_NEEDS_ATTENTION, Message::Text("Needs attention")),
    (TILE_UPDATING, Message::Text("Updating…")),
    (TILE_STOPPING, Message::Text("Stopping…")),
    (TILE_UNINSTALLING, Message::Text("Uninstalling…")),
    (TILE_INSTALLING, Message::Text("Installing…")),
    (TILE_UPDATE, Message::Text("Update")),
    (TILE_INSTALLED, Message::Text("Installed")),
    (TILE_NOT_INSTALLED, Message::Text("Not installed")),
    (TILE_NOT_IN_LIBRARY, Message::Text("Not in library")),
    (VERB_INSTALLING, Message::Text("Installing")),
    (VERB_UPDATING, Message::Text("Updating")),
    (VERB_UNINSTALLING, Message::Text("Uninstalling")),
    (VERB_STOPPING, Message::Text("Stopping")),
    (VERB_DRAINING, Message::Text("Draining")),
    (VERB_STARTING, Message::Text("Starting")),
    (
        SEARCH_SOURCES,
        Message::Plural {
            one: "Searched {count} source",
            other: "Searched {count} sources",
        },
    ),
    (SEARCH_UNAVAILABLE, Message::Text("{count} unavailable")),
    (LIBRARY_FILTER_ALL, Message::Text("All")),
    (LIBRARY_FILTER_RUNNING, Message::Text("Running")),
    (LIBRARY_FILTER_UPDATES, Message::Text("Updates")),
    (POPULAR_TITLE, Message::Text("Popular")),
    (POPULAR_LOADED, Message::Text("{count} loaded")),
    (POPULAR_ALL, Message::Text("All")),
    (
        COLLECTION_ARROWS,
        Message::Plural {
            one: "{count} arrow",
            other: "{count} arrows",
        },
    ),
    (
        COLLECTION_ALL_INSTALLED,
        Message::Text("{count} arrows · all installed"),
    ),
    (
        COLLECTION_UNRESOLVED_TITLE,
        Message::Text("Couldn't be loaded"),
    ),
    (
        COLLECTION_UNRESOLVED_RETRY,
        Message::Text("Retry to load this arrow."),
    ),
    (COLLECTION_FOLLOWING, Message::Text("Following")),
    (COLLECTION_FOLLOW, Message::Text("Follow")),
    (REMOTE_THIS_DEVICE, Message::Text("This device")),
    (REMOTE_REMOTE, Message::Text("Remote")),
    (REMOTE_NOT_CONNECTED, Message::Text("Not connected")),
    (APP_MENU_SETTINGS, Message::Text("Settings")),
    (APP_MENU_METHODS, Message::Text("Methods")),
    (APP_MENU_OPEN_PAGE, Message::Text("Open arrow page")),
    (APP_MENU_UNINSTALL, Message::Text("Uninstall…")),
];
