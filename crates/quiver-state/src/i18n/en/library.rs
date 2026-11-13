//! English messages for the `library.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const SUBTITLE: Key = Key::new("library.subtitle");
pub const SORT_NAME: Key = Key::new("library.sort.name");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (
        SUBTITLE,
        Message::Plural {
            one: "{count} installed arrow",
            other: "{count} installed arrows",
        },
    ),
    (SORT_NAME, Message::Text("Sort: Name")),
];
