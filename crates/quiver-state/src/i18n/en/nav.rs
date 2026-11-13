//! English messages for the `nav.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const HOME: Key = Key::new("nav.home");
pub const REMOTE: Key = Key::new("nav.remote");
pub const SETTINGS: Key = Key::new("nav.settings");
pub const BACK: Key = Key::new("nav.back");
pub const FORWARD: Key = Key::new("nav.forward");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (HOME, Message::Text("Home")),
    (REMOTE, Message::Text("Remote")),
    (SETTINGS, Message::Text("Settings")),
    (BACK, Message::Text("Go back")),
    (FORWARD, Message::Text("Go forward")),
];
