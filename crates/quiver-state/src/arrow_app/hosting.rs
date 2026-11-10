//! Keeping the arrow app's window in step with the pane (the native half of `ArrowAppView`'s
//! `show`/`hide` effect). The page renders in its own OS window (docs/decision-log.md), so "the visible
//! app" is the one whose pane the user is looking at; leaving the pane hides it, never closes it.

/// What the pane asks of the window supervisor after a change of route or readiness.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HostStep {
    /// `show(namespace)`: make this the visible app, opening its window when it has none.
    Show,
    /// `hide()`: nothing is visible any more; the window stays alive.
    Hide,
    Keep,
}

/// `route_namespace` is the app the pane is showing (`None` on any other route). The window opens only
/// once the surface answers: a frame loaded earlier would sit on the daemon's 502 page.
pub fn host_step(route_namespace: Option<&str>, ready: bool, visible: Option<&str>) -> HostStep {
    let wanted = route_namespace.filter(|_| ready);
    match (wanted, visible) {
        (Some(want), Some(now)) if want == now => HostStep::Keep,
        (Some(_), _) => HostStep::Show,
        (None, Some(_)) => HostStep::Hide,
        (None, None) => HostStep::Keep,
    }
}

/// What the pane's body says about the window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaneBody {
    /// The surface is not ready: a spinner and "Starting...".
    Starting,
    /// The window is open.
    Open,
    /// Ready, but there is no window: the user closed it, the cap evicted it, or it could not start.
    Closed,
}

pub fn pane_body(ready: bool, window_open: bool) -> PaneBody {
    match (ready, window_open) {
        (false, _) => PaneBody::Starting,
        (true, true) => PaneBody::Open,
        (true, false) => PaneBody::Closed,
    }
}

/// Which open apps lost their window to the cap, as the names to tell the user about. The names come
/// from the arrow store; an arrow it does not hold is named by its namespace.
pub fn eviction_name(namespace: &str, name: Option<&str>) -> String {
    name.filter(|n| !n.is_empty())
        .unwrap_or(namespace)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ready_app_is_shown_once_and_then_left_alone() {
        assert_eq!(host_step(Some("a"), true, None), HostStep::Show);
        assert_eq!(host_step(Some("a"), true, Some("a")), HostStep::Keep);
        assert_eq!(host_step(Some("b"), true, Some("a")), HostStep::Show);
    }

    #[test]
    fn nothing_opens_before_the_surface_answers() {
        assert_eq!(host_step(Some("a"), false, None), HostStep::Keep);
        assert_eq!(host_step(Some("a"), false, Some("a")), HostStep::Hide);
    }

    #[test]
    fn leaving_the_pane_hides_what_was_visible() {
        assert_eq!(host_step(None, true, Some("a")), HostStep::Hide);
        assert_eq!(host_step(None, true, None), HostStep::Keep);
    }

    #[test]
    fn the_body_follows_readiness_then_the_window() {
        assert_eq!(pane_body(false, false), PaneBody::Starting);
        assert_eq!(pane_body(false, true), PaneBody::Starting);
        assert_eq!(pane_body(true, true), PaneBody::Open);
        assert_eq!(pane_body(true, false), PaneBody::Closed);
    }

    #[test]
    fn an_unknown_arrow_is_named_by_its_namespace() {
        assert_eq!(eviction_name("github.com/x/y", None), "github.com/x/y");
        assert_eq!(eviction_name("github.com/x/y", Some("")), "github.com/x/y");
        assert_eq!(eviction_name("github.com/x/y", Some("Chat")), "Chat");
    }
}
