//! The General/Developer tab's saved choices, in the string store the web app kept in `localStorage`:
//! the theme and rail are `shell`'s; these are the build indicator, whether Quiver's own components are
//! listed, and the language. A value that cannot be read or does not make sense reads as "not chosen".

use super::catalog_visibility::CATALOG_VISIBILITY_STORAGE_KEY;
use crate::console::build_info::{
    BuildChannel, BuildIndicatorSetting, BUILD_INDICATOR_STORAGE_KEY,
};
use crate::i18n::{normalise_preference, LocalePreference};
use crate::persistence::KeyValueStore;

pub const LOCALE_STORAGE_KEY: &str = "quiver.locale";

fn flag(raw: Option<String>) -> Option<bool> {
    match raw.as_deref() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

fn read(
    store: &dyn KeyValueStore,
    key: &str,
) -> Option<String> {
    store.get(key).ok().flatten()
}

fn write(
    store: &dyn KeyValueStore,
    key: &str,
    value: &str,
) {
    if let Err(err) = store.set(key, value) {
        log::warn!("could not save {key}: {err}");
    }
}

/// `None` when the person never chose: the caller's own default applies.
pub fn load_show_self_components(store: &dyn KeyValueStore) -> Option<bool> {
    flag(read(store, CATALOG_VISIBILITY_STORAGE_KEY))
}

pub fn save_show_self_components(
    store: &dyn KeyValueStore,
    show: bool,
) {
    write(store, CATALOG_VISIBILITY_STORAGE_KEY, &show.to_string());
}

pub fn load_build_indicator(
    store: &dyn KeyValueStore,
    channel: BuildChannel,
) -> BuildIndicatorSetting {
    BuildIndicatorSetting::new(flag(read(store, BUILD_INDICATOR_STORAGE_KEY)), channel)
}

/// Choosing the default stores nothing (and clears an older choice).
pub fn save_build_indicator(
    store: &dyn KeyValueStore,
    setting: &BuildIndicatorSetting,
) {
    match setting.stored() {
        Some(shown) => write(store, BUILD_INDICATOR_STORAGE_KEY, &shown.to_string()),
        None => write(store, BUILD_INDICATOR_STORAGE_KEY, ""),
    }
}

pub fn load_locale(store: &dyn KeyValueStore) -> LocalePreference {
    normalise_preference(read(store, LOCALE_STORAGE_KEY).as_deref())
}

pub fn save_locale(
    store: &dyn KeyValueStore,
    preference: LocalePreference,
) {
    write(store, LOCALE_STORAGE_KEY, preference.as_stored());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::InMemoryKeyValueStore;

    #[test]
    fn nothing_stored_means_the_defaults() {
        let store = InMemoryKeyValueStore::new();
        assert_eq!(load_show_self_components(&store), None);
        assert_eq!(load_locale(&store), LocalePreference::System);
        assert!(!load_build_indicator(&store, BuildChannel::Stable).shown());
        assert!(load_build_indicator(&store, BuildChannel::Nightly).shown());
    }

    #[test]
    fn choices_round_trip_and_choosing_the_default_clears_the_choice() {
        let store = InMemoryKeyValueStore::new();
        save_show_self_components(&store, false);
        assert_eq!(load_show_self_components(&store), Some(false));

        let mut setting = load_build_indicator(&store, BuildChannel::Stable);
        setting.set(true);
        save_build_indicator(&store, &setting);
        assert!(load_build_indicator(&store, BuildChannel::Stable).shown());
        setting.set(false);
        save_build_indicator(&store, &setting);
        assert!(!load_build_indicator(&store, BuildChannel::Stable).changed());
    }

    #[test]
    fn garbage_reads_as_not_chosen() {
        let store = InMemoryKeyValueStore::new();
        store.set(CATALOG_VISIBILITY_STORAGE_KEY, "maybe").unwrap();
        store.set(LOCALE_STORAGE_KEY, "klingon").unwrap();
        assert_eq!(load_show_self_components(&store), None);
        assert_eq!(load_locale(&store), LocalePreference::System);
    }
}
