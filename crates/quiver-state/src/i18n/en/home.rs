//! English messages for the `home.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const RECENTS: Key = Key::new("home.recents");
pub const LIBRARY: Key = Key::new("home.library");
pub const COLLECTIONS: Key = Key::new("home.collections");
pub const RECOMMENDED: Key = Key::new("home.recommended");
pub const VIEW_ALL_ARROWS: Key = Key::new("home.viewAllArrows");
pub const VIEW_ALL_RECOMMENDED: Key = Key::new("home.viewAllRecommended");
pub const VIEW_ALL_COLLECTIONS: Key = Key::new("home.viewAllCollections");
pub const EMPTY_TITLE: Key = Key::new("home.empty.title");
pub const EMPTY_DESCRIPTION: Key = Key::new("home.empty.description");
pub const EMPTY_CTA: Key = Key::new("home.empty.cta");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (RECENTS, Message::Text("Recents")),
    (LIBRARY, Message::Text("Library")),
    (COLLECTIONS, Message::Text("Collections")),
    (RECOMMENDED, Message::Text("Recommended")),
    (
        VIEW_ALL_ARROWS,
        Message::Plural {
            one: "View {count} arrow",
            other: "View all {count} arrows",
        },
    ),
    (
        VIEW_ALL_RECOMMENDED,
        Message::Plural {
            one: "View {count} arrow",
            other: "View all {count}",
        },
    ),
    (
        VIEW_ALL_COLLECTIONS,
        Message::Plural {
            one: "View {count} collection",
            other: "View all {count} collections",
        },
    ),
    (EMPTY_TITLE, Message::Text("Nothing here yet")),
    (
        EMPTY_DESCRIPTION,
        Message::Text("Installed arrows and the collections you follow will show up here."),
    ),
    (EMPTY_CTA, Message::Text("Search for arrows")),
];
