//! `QUIVER_DEMO_ROUTE=<route>` opens the demo on another screen (screenshots of screens Home does not
//! link to): `recommended`, `library`, `collections`, `collection:<namespace>`, `search:<query>`,
//! `settings[:engine|developer]`, `remote`, `arrow-app`, `arrow:<namespace>[:settings|:methods]` (the
//! suffix opens that modal once the page has loaded; an unknown namespace is the 404 state, `arrow:error`
//! the load-error state). `QUIVER_DEMO_THEME=dark|light` picks the theme.

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
        "arrow" => Route::Arrow {
            namespace: arrow_target(rest).0.into(),
        },
        "arrow-app" => Route::ArrowApp {
            namespace: super::screens::APP_NAMESPACE.into(),
        },
        _ => return None,
    })
}

/// `<namespace>[:settings|:methods]`: the arrow and the modal to open on it.
fn arrow_target(rest: &str) -> (&str, Option<&str>) {
    match rest.rsplit_once(':') {
        Some((ns, modal @ ("settings" | "methods"))) => (ns, Some(modal)),
        _ => (rest, None),
    }
}

/// The modal the `arrow:<ns>:<modal>` route asks the arrow page to open.
pub fn arrow_modal_from_environment() -> Option<&'static str> {
    let spec = std::env::var("QUIVER_DEMO_ROUTE").ok()?;
    let (kind, rest) = spec.split_once(':')?;
    match (kind == "arrow").then(|| arrow_target(rest).1).flatten()? {
        "methods" => Some("methods"),
        _ => Some("settings"),
    }
}

/// `QUIVER_DEMO_THEME` (`dark` / `light`), for screenshots of both modes.
pub fn theme_from_environment() -> Option<quiver_state::shell::ThemePreference> {
    match std::env::var("QUIVER_DEMO_THEME").ok()?.as_str() {
        "dark" => Some(quiver_state::shell::ThemePreference::Dark),
        "light" => Some(quiver_state::shell::ThemePreference::Light),
        _ => None,
    }
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
        assert_eq!(
            parse("arrow:github.com/a/b:methods"),
            Some(Route::Arrow {
                namespace: "github.com/a/b".into()
            })
        );
        assert_eq!(arrow_target("github.com/a/b:settings").1, Some("settings"));
        assert_eq!(arrow_target("github.com/a/b").1, None);
        assert_eq!(parse("nope"), None);
    }
}
