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

/// Developer is for debug builds only (`debug_assertions`), as `import.meta.env.DEV` was.
pub fn visible_tabs(dev_build: bool) -> Vec<TabItem> {
    let mut tabs = vec![
        TabItem {
            id: SettingsTab::General,
            label: settings::TAB_GENERAL,
        },
        TabItem {
            id: SettingsTab::Engine,
            label: settings::TAB_ENGINE,
        },
    ];
    if dev_build {
        tabs.push(TabItem {
            id: SettingsTab::Developer,
            label: settings::TAB_DEVELOPER,
        });
    }
    tabs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_only_shows_in_a_dev_build_and_general_is_first() {
        assert_eq!(visible_tabs(false).len(), 2);
        let dev = visible_tabs(true);
        assert_eq!(dev.last().map(|t| t.id), Some(SettingsTab::Developer));
        assert_eq!(dev[0].id, SettingsTab::General);
        assert_eq!(SettingsTab::default(), SettingsTab::General);
    }
}
