//! English messages for the `collections.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const SUBTITLE: Key = Key::new("collections.subtitle");
pub const ARROW_COUNT: Key = Key::new("collections.arrowCount");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (SUBTITLE, Message::Text("{count} followed")),
    (
        ARROW_COUNT,
        Message::Plural {
            one: "{count} arrow",
            other: "{count} arrows",
        },
    ),
];
