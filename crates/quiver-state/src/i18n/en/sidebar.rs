//! English messages for the `sidebar.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const RESIZE: Key = Key::new("sidebar.resize");
pub const ARROWS: Key = Key::new("sidebar.arrows");
pub const ARROWS_LOADING: Key = Key::new("sidebar.arrows.loading");
pub const ARROWS_EMPTY_TITLE: Key = Key::new("sidebar.arrows.empty.title");
pub const ARROWS_EMPTY_BODY: Key = Key::new("sidebar.arrows.empty.body");
pub const ARROWS_ERROR_TITLE: Key = Key::new("sidebar.arrows.error.title");
pub const ARROWS_ERROR_BODY: Key = Key::new("sidebar.arrows.error.body");
pub const ARROWS_ERROR_ACTION: Key = Key::new("sidebar.arrows.error.action");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (RESIZE, Message::Text("Resize sidebar")),
    (ARROWS, Message::Text("Arrows")),
    (ARROWS_LOADING, Message::Text("Loading arrows")),
    (ARROWS_EMPTY_TITLE, Message::Text("No arrows yet")),
    (
        ARROWS_EMPTY_BODY,
        Message::Text("Arrows you install show up here."),
    ),
    (ARROWS_ERROR_TITLE, Message::Text("Can’t reach quiver.core")),
    (
        ARROWS_ERROR_BODY,
        Message::Text(
            "Your arrows are still installed. This list fills in once the connection is back.",
        ),
    ),
    (ARROWS_ERROR_ACTION, Message::Text("Check Engine settings")),
];
