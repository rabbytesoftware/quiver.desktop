//! The pane an arrow with its own interface shows (port of `src/features/arrow-app`, the non-rendering
//! half). The open-app cap and eviction (`MAX_OPEN_APPS`, `admit`, `ArrowAppRegistry`), the page origin and
//! the WebSocket bridge's trust and socket scoping live in `quiver-arrow-host`, where they are already
//! ported with their tests; this module decides only what the pane shows.

use crate::domain::ArrowEntry;
use crate::i18n::{
    en::{arrowapp, design},
    Key,
};

/// Namespaces whose arrow currently has an open surface.
pub fn surface_namespaces(arrows: &[ArrowEntry]) -> Vec<String> {
    arrows
        .iter()
        .filter(|a| {
            a.active_run
                .as_ref()
                .is_some_and(|run| run.surface.is_some())
        })
        .map(|a| a.namespace.clone())
        .collect()
}

/// A menu entry of the pane's overflow button.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaneMenuItem {
    Settings,
    Methods,
    OpenArrowPage,
    Uninstall,
}

impl PaneMenuItem {
    pub fn label(self) -> Key {
        match self {
            Self::Settings => design::APP_MENU_SETTINGS,
            Self::Methods => design::APP_MENU_METHODS,
            Self::OpenArrowPage => design::APP_MENU_OPEN_PAGE,
            Self::Uninstall => design::APP_MENU_UNINSTALL,
        }
    }

    /// Uninstall asks first (the trailing ellipsis) and is drawn as destructive.
    pub fn destructive(self) -> bool {
        self == Self::Uninstall
    }
}

/// What the pane's header offers: Reload, Details, Stop, and the overflow menu.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PaneHeader {
    pub name: String,
    pub icon: Option<String>,
    /// "Running" once the page answers, "Starting" before.
    pub status: Key,
    pub ready: bool,
    /// While not ready the pane covers the (not yet mounted) frame with the starting notice. A frame
    /// loaded earlier would sit on the daemon's 502 page.
    pub show_starting: bool,
    pub menu: Vec<PaneMenuItem>,
}

/// `show_settings` and `show_methods` come from the details action row (an arrow with nothing to
/// configure or no methods does not offer them).
pub fn build_pane_header(
    namespace: &str,
    arrow: Option<&ArrowEntry>,
    show_settings: bool,
    show_methods: bool,
) -> PaneHeader {
    let ready = arrow
        .and_then(|a| a.active_run.as_ref())
        .and_then(|run| run.surface.as_ref())
        .is_some_and(|surface| surface.ready);
    let mut menu = Vec::new();
    if show_settings {
        menu.push(PaneMenuItem::Settings);
    }
    if show_methods {
        menu.push(PaneMenuItem::Methods);
    }
    menu.push(PaneMenuItem::OpenArrowPage);
    menu.push(PaneMenuItem::Uninstall);
    PaneHeader {
        name: arrow.map_or_else(|| namespace.to_owned(), |a| a.name.clone()),
        icon: arrow.and_then(|a| a.icon.clone()),
        status: if ready {
            arrowapp::RUNNING
        } else {
            arrowapp::STARTING
        },
        ready,
        show_starting: !ready,
        menu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ActiveRun, ArrowState, ArrowSurface, SurfaceMode};

    fn arrow(surface: Option<ArrowSurface>) -> ArrowEntry {
        ArrowEntry {
            namespace: "github.com/x/app".into(),
            name: "App".into(),
            description: String::new(),
            tags: vec![],
            icon: Some("icon.png".into()),
            banner: None,
            version: "1".into(),
            state: ArrowState::Running,
            active_run: Some(ActiveRun {
                method: "execute".into(),
                pid: None,
                variables: std::collections::BTreeMap::new(),
                steps: vec![],
                surface,
            }),
            last_return: None,
            last_used_at: None,
            origin: None,
            confidence: None,
        }
    }

    fn surface(ready: bool) -> ArrowSurface {
        ArrowSurface {
            mode: SurfaceMode::Listen,
            path: "/".into(),
            ready,
        }
    }

    #[test]
    fn only_arrows_with_an_open_surface_are_live() {
        let with = arrow(Some(surface(false)));
        let without = ArrowEntry {
            namespace: "other".into(),
            ..arrow(None)
        };
        let idle = ArrowEntry {
            namespace: "idle".into(),
            active_run: None,
            ..arrow(None)
        };
        assert_eq!(
            surface_namespaces(&[with, without, idle]),
            ["github.com/x/app"]
        );
    }

    #[test]
    fn the_header_says_starting_until_the_page_answers() {
        let starting = build_pane_header("ns", Some(&arrow(Some(surface(false)))), true, true);
        assert_eq!(
            (starting.status, starting.show_starting),
            (arrowapp::STARTING, true)
        );
        let running = build_pane_header("ns", Some(&arrow(Some(surface(true)))), true, true);
        assert_eq!(
            (running.status, running.show_starting, running.ready),
            (arrowapp::RUNNING, false, true)
        );
    }

    #[test]
    fn the_header_falls_back_to_the_namespace_for_an_arrow_the_store_does_not_hold() {
        let h = build_pane_header("github.com/x/gone", None, false, false);
        assert_eq!(
            (h.name.as_str(), h.icon, h.ready),
            ("github.com/x/gone", None, false)
        );
    }

    #[test]
    fn the_menu_offers_settings_and_methods_only_when_the_arrow_has_them_and_uninstall_last() {
        let all = build_pane_header("ns", None, true, true).menu;
        assert_eq!(
            all,
            [
                PaneMenuItem::Settings,
                PaneMenuItem::Methods,
                PaneMenuItem::OpenArrowPage,
                PaneMenuItem::Uninstall
            ]
        );
        let bare = build_pane_header("ns", None, false, false).menu;
        assert_eq!(bare, [PaneMenuItem::OpenArrowPage, PaneMenuItem::Uninstall]);
        assert!(PaneMenuItem::Uninstall.destructive() && !PaneMenuItem::Settings.destructive());
    }
}
