//! The status badge on a tile's art.

use crate::arrow_details::status::{RunState, StatusGlyph};
use crate::i18n::{en::design, Key};

/// Which text the badge carries. `Custom` is a label the caller owns (e.g. "12 arrows" on a collection).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TileBadgeKind {
    Running,
    NeedsAttention,
    Updating,
    Stopping,
    Uninstalling,
    Installing,
    Update,
    Installed,
    NotInstalled,
    Custom(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TileBadge {
    pub kind: TileBadgeKind,
    /// The glyph in front of the label: spinner while busy, dotted square when running, ...
    pub glyph: Option<StatusGlyph>,
}

impl TileBadge {
    /// The label's catalogue key, or `None` for a custom label.
    pub fn label(&self) -> Option<Key> {
        Some(match self.kind {
            TileBadgeKind::Running => design::TILE_RUNNING,
            TileBadgeKind::NeedsAttention => design::TILE_NEEDS_ATTENTION,
            TileBadgeKind::Updating => design::TILE_UPDATING,
            TileBadgeKind::Stopping => design::TILE_STOPPING,
            TileBadgeKind::Uninstalling => design::TILE_UNINSTALLING,
            TileBadgeKind::Installing => design::TILE_INSTALLING,
            TileBadgeKind::Update => design::TILE_UPDATE,
            TileBadgeKind::Installed => design::TILE_INSTALLED,
            TileBadgeKind::NotInstalled => design::TILE_NOT_INSTALLED,
            TileBadgeKind::Custom(_) => return None,
        })
    }
}

/// Whether the steady states get a badge. The screens that list only your own arrows (Home, Library)
/// stay quiet about the unremarkable case; Search and Collection mix installed and not-installed
/// arrows, so there "Installed" / "Not installed" are information.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BadgePolicy {
    ActivityOnly,
    WithLibraryState,
}

pub fn badge_for(
    state: RunState,
    user_installed: bool,
    update_ahead: bool,
    policy: BadgePolicy,
) -> Option<TileBadge> {
    use TileBadgeKind as K;
    let busy = Some(StatusGlyph::Busy);
    if !user_installed {
        return (policy == BadgePolicy::WithLibraryState).then_some(TileBadge {
            kind: K::NotInstalled,
            glyph: None,
        });
    }
    let (kind, glyph) = match state {
        RunState::Running => (K::Running, Some(StatusGlyph::Running)),
        RunState::Detached => (K::NeedsAttention, Some(StatusGlyph::NeedsAttention)),
        RunState::Updating => (K::Updating, busy),
        RunState::Stopping | RunState::Draining => (K::Stopping, busy),
        RunState::Uninstalling => (K::Uninstalling, busy),
        RunState::Installing => (K::Installing, busy),
        RunState::Outdated => (K::Update, Some(StatusGlyph::UpdateAvailable)),
        RunState::Ready if update_ahead => (K::Update, Some(StatusGlyph::UpdateAvailable)),
        RunState::Ready | RunState::Absent | RunState::Removed => {
            return (policy == BadgePolicy::WithLibraryState && state == RunState::Ready)
                .then_some(TileBadge {
                    kind: K::Installed,
                    glyph: None,
                });
        }
    };
    Some(TileBadge { kind, glyph })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(
        state: RunState,
        installed: bool,
        ahead: bool,
        policy: BadgePolicy,
    ) -> Option<TileBadgeKind> {
        badge_for(state, installed, ahead, policy).map(|b| b.kind)
    }

    #[test]
    fn activity_states_always_get_a_badge() {
        use TileBadgeKind::*;
        let p = BadgePolicy::ActivityOnly;
        assert_eq!(kind(RunState::Running, true, false, p), Some(Running));
        assert_eq!(
            kind(RunState::Detached, true, false, p),
            Some(NeedsAttention)
        );
        assert_eq!(kind(RunState::Updating, true, false, p), Some(Updating));
        assert_eq!(kind(RunState::Stopping, true, false, p), Some(Stopping));
        assert_eq!(kind(RunState::Draining, true, false, p), Some(Stopping));
        assert_eq!(
            kind(RunState::Uninstalling, true, false, p),
            Some(Uninstalling)
        );
        assert_eq!(kind(RunState::Installing, true, false, p), Some(Installing));
        assert_eq!(kind(RunState::Outdated, true, false, p), Some(Update));
        assert_eq!(kind(RunState::Ready, true, true, p), Some(Update));
    }

    #[test]
    fn steady_states_are_silent_unless_the_screen_mixes_library_state() {
        let quiet = BadgePolicy::ActivityOnly;
        assert_eq!(kind(RunState::Ready, true, false, quiet), None);
        assert_eq!(kind(RunState::Absent, true, false, quiet), None);
        assert_eq!(kind(RunState::Absent, false, false, quiet), None);
        let mixed = BadgePolicy::WithLibraryState;
        assert_eq!(
            kind(RunState::Ready, true, false, mixed),
            Some(TileBadgeKind::Installed)
        );
        assert_eq!(
            kind(RunState::Absent, false, false, mixed),
            Some(TileBadgeKind::NotInstalled)
        );
        assert_eq!(kind(RunState::Absent, true, false, mixed), None);
    }

    #[test]
    fn a_row_outside_the_library_ignores_its_runtime_state() {
        assert_eq!(
            kind(RunState::Running, false, false, BadgePolicy::ActivityOnly),
            None
        );
    }

    #[test]
    fn glyphs_follow_the_status_vocabulary() {
        let glyph = |s| badge_for(s, true, false, BadgePolicy::ActivityOnly).and_then(|b| b.glyph);
        assert_eq!(glyph(RunState::Installing), Some(StatusGlyph::Busy));
        assert_eq!(glyph(RunState::Running), Some(StatusGlyph::Running));
        assert_eq!(glyph(RunState::Detached), Some(StatusGlyph::NeedsAttention));
        assert_eq!(
            glyph(RunState::Outdated),
            Some(StatusGlyph::UpdateAvailable)
        );
    }

    #[test]
    fn labels_resolve_through_the_catalogue() {
        let badge = badge_for(RunState::Updating, true, false, BadgePolicy::ActivityOnly).unwrap();
        assert_eq!(
            crate::i18n::Translator::default().t(badge.label().unwrap()),
            "Updating…"
        );
        assert!(TileBadge {
            kind: TileBadgeKind::Custom("3 arrows".into()),
            glyph: None
        }
        .label()
        .is_none());
    }
}
