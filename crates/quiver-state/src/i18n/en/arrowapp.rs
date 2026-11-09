//! English messages for the `arrowApp.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const RELOAD: Key = Key::new("arrowApp.reload");
pub const STOP: Key = Key::new("arrowApp.stop");
pub const DETAILS: Key = Key::new("arrowApp.details");
pub const STARTING: Key = Key::new("arrowApp.starting");
pub const RUNNING: Key = Key::new("arrowApp.running");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (RELOAD, Message::Text("Reload")),
    (STOP, Message::Text("Stop")),
    (DETAILS, Message::Text("Details")),
    (STARTING, Message::Text("Starting...")),
    (RUNNING, Message::Text("Running")),
];
