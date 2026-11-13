//! Where the user is. Mirrors the router paths of the Tauri app, so the sidebar's decisions
//! (`quiver_state::sidebar::active_destination`) keep working on `path()`.

use quiver_state::settings::SettingsTab;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum Route {
    #[default]
    Home,
    Recommended,
    Search {
        query: String,
    },
    Library,
    Collections,
    Collection {
        id: String,
    },
    Arrow {
        namespace: String,
    },
    Remote,
    Settings {
        tab: SettingsTab,
    },
}

impl Route {
    /// The router path without a query string.
    pub fn path(&self) -> String {
        match self {
            Self::Home => "/".into(),
            Self::Recommended => "/recommended".into(),
            Self::Search { .. } => "/search".into(),
            Self::Library => "/library".into(),
            Self::Collections => "/collections".into(),
            Self::Collection { id } => format!("/collections/{id}"),
            Self::Arrow { namespace } => format!("/arrow/{namespace}"),
            Self::Remote => "/remote".into(),
            Self::Settings {
                tab: SettingsTab::General,
            } => "/settings".into(),
            Self::Settings {
                tab: SettingsTab::Engine,
            } => "/settings/engine".into(),
            Self::Settings {
                tab: SettingsTab::Developer,
            } => "/settings/developer".into(),
        }
    }

    /// The e2e screen name (`screen.<name>`).
    pub fn screen(&self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Recommended => "recommended",
            Self::Search { .. } => "search",
            Self::Library => "library",
            Self::Collections => "collections",
            Self::Collection { .. } => "collection",
            Self::Arrow { .. } => "arrow",
            Self::Remote => "remote",
            Self::Settings { .. } => "settings",
        }
    }

    /// The arrow whose page this is, for the sidebar's selected row.
    pub fn arrow_namespace(&self) -> Option<&str> {
        match self {
            Self::Arrow { namespace } => Some(namespace),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quiver_state::sidebar::{active_destination, Destination};

    #[test]
    fn pages_that_hang_off_home_keep_home_lit() {
        for route in [
            Route::Home,
            Route::Library,
            Route::Collections,
            Route::Recommended,
        ] {
            assert_eq!(active_destination(&route.path()), Some(Destination::Home));
        }
        let engine = Route::Settings {
            tab: SettingsTab::Engine,
        };
        assert_eq!(
            active_destination(&engine.path()),
            Some(Destination::Settings)
        );
    }

    #[test]
    fn an_arrow_page_lights_no_tab_but_names_its_arrow() {
        let route = Route::Arrow {
            namespace: "github.com/a/b".into(),
        };
        assert_eq!(active_destination(&route.path()), None);
        assert_eq!(route.arrow_namespace(), Some("github.com/a/b"));
    }
}
