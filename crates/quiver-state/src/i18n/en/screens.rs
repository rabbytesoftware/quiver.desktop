//! Strings of the settings, remote, console and arrow-app screens that the boards added or reworded
//! (the pointer to the Developer tab, the remote subtitle, the hosting pane, the console's controls).

use crate::i18n::message::{Key, Message};

pub const REMOTE_SUBTITLE: Key = Key::new("screens.remote.subtitle");
pub const REMOTE_LOCAL_CAPTION: Key = Key::new("screens.remote.localCaption");
pub const SETTINGS_DEVELOPER_POINTER: Key = Key::new("screens.settings.developerPointer");
pub const SETTINGS_CHANNEL_FAILED: Key = Key::new("screens.settings.channelFailed");
pub const SETTINGS_ABOUT_TITLE: Key = Key::new("screens.settings.about.title");
pub const SETTINGS_ABOUT_APP: Key = Key::new("screens.settings.about.app");
pub const SETTINGS_ABOUT_CORE: Key = Key::new("screens.settings.about.core");
pub const SETTINGS_DESKTOP_UPDATE: Key = Key::new("screens.settings.desktopUpdate");
pub const SETTINGS_CORE_UPDATE: Key = Key::new("screens.settings.coreUpdate");
pub const SETTINGS_DESKTOP_RESTARTS: Key = Key::new("screens.settings.desktopRestarts");
pub const SETTINGS_DEVELOPER_NOTE: Key = Key::new("screens.settings.developerNote");
pub const ARROWAPP_WINDOW_CLOSED: Key = Key::new("screens.arrowApp.windowClosed");
pub const ARROWAPP_OPEN_WINDOW: Key = Key::new("screens.arrowApp.openWindow");
pub const ARROWAPP_EVICTED: Key = Key::new("screens.arrowApp.evicted");
pub const ARROWAPP_FAILED: Key = Key::new("screens.arrowApp.failed");
pub const CONSOLE_CANCEL: Key = Key::new("screens.console.cancel");
pub const CONSOLE_CLEAR: Key = Key::new("screens.console.clear");
pub const CONSOLE_RUNNING: Key = Key::new("screens.console.running");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (
        REMOTE_SUBTITLE,
        Message::Text("Your devices and saved connections."),
    ),
    (REMOTE_LOCAL_CAPTION, Message::Text("This computer")),
    (
        SETTINGS_DEVELOPER_POINTER,
        Message::Text(
            "Build indicator, Daemon console and Quiver’s own components are in the Developer tab.",
        ),
    ),
    (
        SETTINGS_CHANNEL_FAILED,
        Message::Text("Quiver Core didn’t answer, so the channels couldn’t load."),
    ),
    (SETTINGS_ABOUT_TITLE, Message::Text("About")),
    (SETTINGS_ABOUT_APP, Message::Text("Quiver Desktop")),
    (SETTINGS_ABOUT_CORE, Message::Text("Quiver Core")),
    (
        SETTINGS_DESKTOP_UPDATE,
        Message::Text("Quiver Desktop updates"),
    ),
    (SETTINGS_CORE_UPDATE, Message::Text("Quiver Core updates")),
    (
        SETTINGS_DESKTOP_RESTARTS,
        Message::Text("Quiver restarts to finish updating."),
    ),
    (
        SETTINGS_DEVELOPER_NOTE,
        Message::Text("Tools for building and debugging Quiver."),
    ),
    (
        ARROWAPP_WINDOW_CLOSED,
        Message::Text("The page was closed. The app is still running."),
    ),
    (ARROWAPP_OPEN_WINDOW, Message::Text("Open page")),
    (
        ARROWAPP_EVICTED,
        Message::Text("{name} was closed to keep at most {max} apps open."),
    ),
    (
        ARROWAPP_FAILED,
        Message::Text("Couldn’t open the window: {reason}"),
    ),
    (CONSOLE_CANCEL, Message::Text("Cancel")),
    (CONSOLE_CLEAR, Message::Text("Clear")),
    (CONSOLE_RUNNING, Message::Text("Running…")),
];
