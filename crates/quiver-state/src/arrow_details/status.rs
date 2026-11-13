//! Runtime state of an arrow and what the UI shows for it (badge label, status glyph, problem chip).
//! Port of `lib/status.ts`, with the glyph vocabulary of the new design.

use crate::i18n::{en::arrow, en::design, Key};

/// quiver.core's runtime state of an arrow row.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RunState {
    #[default]
    Absent,
    Installing,
    Updating,
    Ready,
    Running,
    Stopping,
    Draining,
    Detached,
    Uninstalling,
    Removed,
    Outdated,
}

impl From<crate::domain::ArrowState> for RunState {
    fn from(state: crate::domain::ArrowState) -> Self {
        use crate::domain::ArrowState as S;
        match state {
            S::Absent => Self::Absent,
            S::Installing => Self::Installing,
            S::Updating => Self::Updating,
            S::Ready => Self::Ready,
            S::Running => Self::Running,
            S::Stopping => Self::Stopping,
            S::Draining => Self::Draining,
            S::Detached => Self::Detached,
            S::Uninstalling => Self::Uninstalling,
            S::Removed => Self::Removed,
            S::Outdated => Self::Outdated,
        }
    }
}

impl RunState {
    /// Parses the wire value; anything unknown reads as `Absent` (the old `default:` branch).
    pub fn parse(value: &str) -> Self {
        match value {
            "installing" => Self::Installing,
            "updating" => Self::Updating,
            "ready" => Self::Ready,
            "running" => Self::Running,
            "stopping" => Self::Stopping,
            "draining" => Self::Draining,
            "detached" => Self::Detached,
            "uninstalling" => Self::Uninstalling,
            "removed" => Self::Removed,
            "outdated" => Self::Outdated,
            _ => Self::Absent,
        }
    }

    /// A lifecycle action is in flight (installing, updating, stopping, draining, uninstalling).
    pub fn is_busy(self) -> bool {
        matches!(
            self,
            Self::Installing
                | Self::Updating
                | Self::Stopping
                | Self::Draining
                | Self::Uninstalling
        )
    }
}

/// The signal icon next to a status, in the new design.
///
/// Busy is the FlickerSpinner, Running the 3x3 dotted square fading along the diagonal;
/// NeedsAttention and UpdateAvailable keep dotted glyphs. A `detached` process is a
/// NeedsAttention (the runtime lost track of it), so it has no glyph of its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusGlyph {
    Busy,
    Running,
    NeedsAttention,
    UpdateAvailable,
    /// Installed and at rest.
    Ready,
    /// Not in the library, or not installed/removed: nothing to signal.
    Inactive,
}

impl StatusGlyph {
    pub fn for_state(
        state: RunState,
        user_installed: bool,
        update_ahead: bool,
    ) -> Self {
        if !user_installed {
            return Self::Inactive;
        }
        if state == RunState::Ready && update_ahead {
            return Self::UpdateAvailable;
        }
        match state {
            RunState::Installing
            | RunState::Updating
            | RunState::Stopping
            | RunState::Draining
            | RunState::Uninstalling => Self::Busy,
            RunState::Running => Self::Running,
            RunState::Detached => Self::NeedsAttention,
            RunState::Outdated => Self::UpdateAvailable,
            RunState::Ready => Self::Ready,
            RunState::Absent | RunState::Removed => Self::Inactive,
        }
    }
}

/// The header's status badge: a translated label and its glyph.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ArrowStatus {
    pub label: Key,
    pub glyph: StatusGlyph,
}

/// "Discovered" (browsed, never added) is not a runtime state, it is `user_installed == false`.
/// A ready row with something `available` reads as "Update available" even before the runtime's own
/// `outdated` state catches up. Every other state maps 1:1.
pub fn compute_status(
    state: RunState,
    user_installed: bool,
    update_ahead: bool,
) -> ArrowStatus {
    let glyph = StatusGlyph::for_state(state, user_installed, update_ahead);
    let label = if !user_installed {
        arrow::STATE_DISCOVERED
    } else if state == RunState::Ready && update_ahead {
        arrow::STATE_OUTDATED
    } else {
        match state {
            RunState::Absent => arrow::STATE_ABSENT,
            RunState::Installing => arrow::STATE_INSTALLING,
            RunState::Ready => arrow::STATE_READY,
            RunState::Outdated => arrow::STATE_OUTDATED,
            RunState::Updating => arrow::STATE_UPDATING,
            RunState::Running => arrow::STATE_RUNNING,
            RunState::Stopping => arrow::STATE_STOPPING,
            RunState::Draining => arrow::STATE_DRAINING,
            RunState::Detached => design::STATUS_NEEDS_ATTENTION,
            RunState::Uninstalling => arrow::STATE_UNINSTALLING,
            RunState::Removed => arrow::STATE_REMOVED,
        }
    };
    ArrowStatus { label, glyph }
}

/// A "problem" chip: a failed run or a detached process.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ArrowProblem {
    Detached,
    /// `detail` is the failed step's own error text from core (technical wire text, shown verbatim);
    /// absent, the caller shows a generic message.
    Failed {
        detail: Option<String>,
    },
}

/// The outcome of the last run, as the detail read reports it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LastRunFacts {
    pub failed: bool,
    /// The first failed step's `error`, if any.
    pub failed_step_error: Option<String>,
}

pub fn problem_message(
    state: RunState,
    last_return: Option<&LastRunFacts>,
) -> Option<ArrowProblem> {
    if state == RunState::Detached {
        return Some(ArrowProblem::Detached);
    }
    let last = last_return.filter(|l| l.failed)?;
    Some(ArrowProblem::Failed {
        detail: last.failed_step_error.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(
        state: RunState,
        installed: bool,
        ahead: bool,
    ) -> Key {
        compute_status(state, installed, ahead).label
    }

    #[test]
    fn a_row_outside_the_library_is_discovered_whatever_its_state() {
        let status = compute_status(RunState::Running, false, false);
        assert_eq!(status.label, arrow::STATE_DISCOVERED);
        assert_eq!(status.glyph, StatusGlyph::Inactive);
    }

    #[test]
    fn a_ready_row_with_something_ahead_reads_as_update_available() {
        let status = compute_status(RunState::Ready, true, true);
        assert_eq!(status.label, arrow::STATE_OUTDATED);
        assert_eq!(status.glyph, StatusGlyph::UpdateAvailable);
        assert_eq!(label(RunState::Ready, true, false), arrow::STATE_READY);
    }

    #[test]
    fn every_state_maps_to_its_glyph() {
        use RunState::*;
        let glyph = |s| compute_status(s, true, false).glyph;
        for busy in [Installing, Updating, Stopping, Draining, Uninstalling] {
            assert_eq!(glyph(busy), StatusGlyph::Busy);
            assert!(busy.is_busy());
        }
        assert_eq!(glyph(Running), StatusGlyph::Running);
        assert_eq!(glyph(Detached), StatusGlyph::NeedsAttention);
        assert_eq!(glyph(Outdated), StatusGlyph::UpdateAvailable);
        assert_eq!(glyph(Ready), StatusGlyph::Ready);
        assert_eq!(glyph(Absent), StatusGlyph::Inactive);
        assert_eq!(glyph(Removed), StatusGlyph::Inactive);
    }

    #[test]
    fn detached_is_labelled_needs_attention() {
        assert_eq!(
            label(RunState::Detached, true, false),
            design::STATUS_NEEDS_ATTENTION
        );
    }

    #[test]
    fn unknown_wire_states_read_as_absent() {
        assert_eq!(RunState::parse("ready"), RunState::Ready);
        assert_eq!(RunState::parse("something-new"), RunState::Absent);
    }

    #[test]
    fn problem_is_detached_or_a_failed_run() {
        assert_eq!(
            problem_message(RunState::Detached, None),
            Some(ArrowProblem::Detached)
        );
        let failed = LastRunFacts {
            failed: true,
            failed_step_error: Some("exit 1".into()),
        };
        assert_eq!(
            problem_message(RunState::Ready, Some(&failed)),
            Some(ArrowProblem::Failed {
                detail: Some("exit 1".into())
            })
        );
        let ok = LastRunFacts {
            failed: false,
            failed_step_error: None,
        };
        assert_eq!(problem_message(RunState::Ready, Some(&ok)), None);
        assert_eq!(problem_message(RunState::Ready, None), None);
    }
}
