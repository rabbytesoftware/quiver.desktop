//! English messages for the `mock.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const BADGE: Key = Key::new("mock.badge");
pub const STATUS: Key = Key::new("mock.status");
pub const TURN_OFF: Key = Key::new("mock.turnOff");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (BADGE, Message::Text("Mock")),
    (
        STATUS,
        Message::Text("{scenario} · no daemon is being contacted"),
    ),
    (TURN_OFF, Message::Text("Turn off")),
];
