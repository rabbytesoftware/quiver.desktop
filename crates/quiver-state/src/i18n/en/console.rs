//! English messages for the `console.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const INDICATOR_LABEL: Key = Key::new("console.indicator.label");
pub const INDICATOR_CORE: Key = Key::new("console.indicator.core");
pub const INDICATOR_APP: Key = Key::new("console.indicator.app");
pub const PANEL_LABEL: Key = Key::new("console.panel.label");
pub const LOG_LABEL: Key = Key::new("console.log.label");
pub const PROMPT_LABEL: Key = Key::new("console.prompt.label");
pub const PROMPT_PLACEHOLDER: Key = Key::new("console.prompt.placeholder");
pub const STREAM_RECONNECTING: Key = Key::new("console.stream.reconnecting");
pub const UNSUPPORTED: Key = Key::new("console.unsupported");
pub const NOTE_RESTARTED: Key = Key::new("console.note.restarted");
pub const NOTE_EXIT: Key = Key::new("console.note.exit");
pub const NOTE_EXIT_WITH_ERROR: Key = Key::new("console.note.exitWithError");
pub const NOTE_REFUSED: Key = Key::new("console.note.refused");
pub const NOTE_UNSUPPORTED: Key = Key::new("console.note.unsupported");
pub const NOTE_HELP_UNAVAILABLE: Key = Key::new("console.note.helpUnavailable");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (INDICATOR_LABEL, Message::Text("Builds and daemon console")),
    (INDICATOR_CORE, Message::Text("core")),
    (INDICATOR_APP, Message::Text("app")),
    (PANEL_LABEL, Message::Text("Daemon console")),
    (LOG_LABEL, Message::Text("Daemon log")),
    (PROMPT_LABEL, Message::Text("Command")),
    (
        PROMPT_PLACEHOLDER,
        Message::Text("info github.com/rabbytesoftware/quiver.core"),
    ),
    (STREAM_RECONNECTING, Message::Text("Reconnecting…")),
    (
        UNSUPPORTED,
        Message::Text("This daemon has no console. Update quiver.core to use it."),
    ),
    (NOTE_RESTARTED, Message::Text("Daemon restarted")),
    (NOTE_EXIT, Message::Text("Exited with code {code}")),
    (
        NOTE_EXIT_WITH_ERROR,
        Message::Text("Exited with code {code}: {error}"),
    ),
    (NOTE_REFUSED, Message::Text("Refused ({status}): {message}")),
    (
        NOTE_UNSUPPORTED,
        Message::Text("This daemon has no console."),
    ),
    (
        NOTE_HELP_UNAVAILABLE,
        Message::Text("The command list is not available."),
    ),
];
