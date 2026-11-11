//! The tabs of the settings screen.

use crate::i18n::{en::settings, Key};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Engine,
    Developer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TabItem {
    pub id: SettingsTab,
    pub label: Key,
}

/// Every build shows all three: the boards moved real settings into Developer (Fletcher's auto-register, the build
/// indicator, the console, Quiver's own components). Only its mock-daemon controls stay debug-only (the app's
/// settings view decides that).
pub fn visible_tabs() -> Vec<TabItem> {
    vec![
        TabItem {
            id: SettingsTab::General,
            label: settings::TAB_GENERAL,
        },
        TabItem {
            id: SettingsTab::Engine,
            label: settings::TAB_ENGINE,
        },
        TabItem {
            id: SettingsTab::Developer,
            label: settings::TAB_DEVELOPER,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_shows_in_every_build_and_general_is_first() {
        let dev = visible_tabs();
        assert_eq!(dev.len(), 3);
        assert_eq!(dev.last().map(|t| t.id), Some(SettingsTab::Developer));
        assert_eq!(dev[0].id, SettingsTab::General);
        assert_eq!(SettingsTab::default(), SettingsTab::General);
    }
}
