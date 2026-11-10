//! `QUIVER_DEMO_ROUTE=<route>` opens the demo on another screen (screenshots of screens Home does not
//! link to): `recommended`, `library`, `collections`, `collection:<namespace>`, `search:<query>`.

use crate::model::Route;

pub fn parse(spec: &str) -> Option<Route> {
    let (kind, rest) = spec.split_once(':').unwrap_or((spec, ""));
    Some(match kind {
        "recommended" => Route::Recommended,
        "library" => Route::Library,
        "collections" => Route::Collections,
        "collection" => Route::Collection { id: rest.into() },
        "search" => Route::Search { query: rest.into() },
        "settings" => Route::Settings {
            tab: match rest {
                "engine" => quiver_state::settings::SettingsTab::Engine,
                "developer" => quiver_state::settings::SettingsTab::Developer,
                _ => quiver_state::settings::SettingsTab::General,
            },
        },
        "remote" => Route::Remote,
        "arrow-app" => Route::ArrowApp {
            namespace: super::screens::APP_NAMESPACE.into(),
        },
        _ => return None,
    })
}

/// The route the environment asks for, if any.
pub fn from_environment() -> Option<Route> {
    parse(&std::env::var("QUIVER_DEMO_ROUTE").ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_map_to_routes() {
        assert_eq!(parse("library"), Some(Route::Library));
        assert_eq!(
            parse("search:rust"),
            Some(Route::Search {
                query: "rust".into()
            })
        );
        assert_eq!(
            parse("collection:github.com/a/b"),
            Some(Route::Collection {
                id: "github.com/a/b".into()
            })
        );
        assert_eq!(parse("nope"), None);
    }
}
