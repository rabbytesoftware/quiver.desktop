//! English messages for the `recommended.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const SUBTITLE: Key = Key::new("recommended.subtitle");
pub const EMPTY: Key = Key::new("recommended.empty");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (
        SUBTITLE,
        Message::Plural {
            one: "{count} recommended arrow",
            other: "{count} recommended arrows",
        },
    ),
    (EMPTY, Message::Text("No recommendations right now.")),
];
