//! English messages for the `app.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const SETTINGS: Key = Key::new("app.settings");
pub const CRASH_TITLE: Key = Key::new("app.crash.title");
pub const CRASH_BODY: Key = Key::new("app.crash.body");
pub const CRASH_RETRY: Key = Key::new("app.crash.retry");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (SETTINGS, Message::Text("Settings")),
    (CRASH_TITLE, Message::Text("Something went wrong")),
    (
        CRASH_BODY,
        Message::Text("This screen failed to render. The rest of Quiver is unaffected."),
    ),
    (CRASH_RETRY, Message::Text("Try again")),
];
