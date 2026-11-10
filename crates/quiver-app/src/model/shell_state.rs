//! The persisted shell values (rail width and side, theme preference) in the string store.

use quiver_state::persistence::KeyValueStore;
use quiver_state::shell::{
    normalise_theme_preference, ShellSettings, ThemePreference, SHELL_STORAGE_KEY,
    THEME_STORAGE_KEY,
};
use serde_json::{json, Value};

pub fn load(store: &dyn KeyValueStore) -> (ShellSettings, ThemePreference) {
    let shell: Value = store
        .get(SHELL_STORAGE_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null);
    let settings = ShellSettings::from_stored(
        shell.get("sidebarSide").and_then(Value::as_str),
        shell.get("sidebarWidth").and_then(Value::as_f64),
    );
    let theme = normalise_theme_preference(store.get(THEME_STORAGE_KEY).ok().flatten().as_deref());
    (settings, theme)
}

pub fn save_shell(
    store: &dyn KeyValueStore,
    settings: &ShellSettings,
) {
    let value = json!({
        "sidebarSide": settings.sidebar_side.as_stored(),
        "sidebarWidth": settings.sidebar_width,
    });
    if let Err(err) = store.set(SHELL_STORAGE_KEY, &value.to_string()) {
        log::warn!("could not save the shell settings: {err}");
    }
}
