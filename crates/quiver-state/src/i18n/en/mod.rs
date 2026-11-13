//! The English catalogue, one file per key group (`nav.*`, `arrow.*`, ...).

use crate::i18n::message::{Key, Message};

pub mod app;
pub mod arrow;
pub mod arrowapp;
pub mod collections;
pub mod console;
pub mod design;
pub mod home;
pub mod library;
pub mod locale_meta;
pub mod mock;
pub mod nav;
pub mod recommended;
pub mod remote;
pub mod screens;
pub mod search;
pub mod settings;
pub mod sidebar;

const GROUPS: &[&[(Key, Message)]] = &[
    locale_meta::MESSAGES,
    app::MESSAGES,
    nav::MESSAGES,
    search::MESSAGES,
    sidebar::MESSAGES,
    arrow::MESSAGES,
    arrowapp::MESSAGES,
    settings::MESSAGES,
    console::MESSAGES,
    design::MESSAGES,
    mock::MESSAGES,
    home::MESSAGES,
    library::MESSAGES,
    recommended::MESSAGES,
    collections::MESSAGES,
    remote::MESSAGES,
    screens::MESSAGES,
];

pub(super) fn lookup(key: Key) -> Option<Message> {
    GROUPS
        .iter()
        .flat_map(|group| group.iter())
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, message)| *message)
}

#[cfg(test)]
pub(super) fn all() -> impl Iterator<Item = &'static (Key, Message)> {
    GROUPS.iter().flat_map(|group| group.iter())
}
