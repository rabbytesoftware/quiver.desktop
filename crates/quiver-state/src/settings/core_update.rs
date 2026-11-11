//! The engine tab's quiver.core self-update rows: its channels and the check/update controls.
//! quiver.core updates like any other arrow; its update ends with the daemon restarting on the same
//! socket, which this panel does not model (the connection layer shows the drop).

use crate::arrow_details::channel::ChannelFacts;
use crate::arrow_details::invoke::ActionFailure;
use crate::arrow_details::status::{ArrowProblem, RunState};

/// quiver.core's own release channels (`GET /v0/arrow/<core>/channels`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CoreChannelsState {
    pub channels: Vec<ChannelFacts>,
    pub loading: bool,
    /// The fetch itself failed, most commonly because quiver.core has not finished self-registering on a
    /// fresh daemon.
    pub failed: bool,
}

impl Default for CoreChannelsState {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
            loading: true,
            failed: false,
        }
    }
}

impl CoreChannelsState {
    pub fn loaded(result: Result<Vec<ChannelFacts>, ActionFailure>) -> Self {
        match result {
            Ok(channels) => Self {
                channels,
                loading: false,
                failed: false,
            },
            Err(_) => Self {
                channels: Vec::new(),
                loading: false,
                failed: true,
            },
        }
    }

    /// The select is usable only with channels to choose from.
    pub fn select_enabled(&self) -> bool {
        !self.loading && !self.failed && !self.channels.is_empty()
    }
}

/// 422 is core refusing for a reason a second attempt will not change; transport failures, 429 and 5xx
/// are worth another try.
pub fn is_retryable(failure: &ActionFailure) -> bool {
    match failure.status {
        None => true,
        Some(status) => status == 429 || status >= 500,
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CoreUpdateNote {
    /// The update call or recheck failed: its message.
    Failure(String),
    /// "Restarts when it finishes."
    Restarts,
    /// An update core accepted can still fail afterwards; its own run records why (`None` text: generic).
    FailedRun(Option<String>),
    None,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CoreUpdateRows {
    pub installed: Option<String>,
    /// `None` reads as "up to date".
    pub available: Option<String>,
    pub note: CoreUpdateNote,
    pub show_retry: bool,
    /// Retry reruns the update when the update itself failed, else the check.
    pub retry_is_update: bool,
    pub check_enabled: bool,
    pub show_update: bool,
    pub update_enabled: bool,
}

pub struct CoreUpdateInput<'a> {
    pub resolved_ref: &'a str,
    pub available_ref: Option<&'a str>,
    pub state: RunState,
    pub update_pending: bool,
    pub recheck_pending: bool,
    pub update_error: Option<&'a ActionFailure>,
    pub recheck_error: Option<&'a ActionFailure>,
    pub problem: Option<&'a ArrowProblem>,
}

/// `stable-26.5.91` and `v26.5.91` (or a tag that ends the resolved ref) name the same release.
fn same_release(
    available: &str,
    resolved: &str,
) -> bool {
    let bare = |s: &str| {
        let s = s.trim();
        let s = s.rsplit_once('@').map_or(s, |(_, r)| r);
        s.trim_start_matches(|c: char| !c.is_ascii_digit())
            .to_owned()
    };
    !resolved.is_empty() && (available == resolved || bare(available) == bare(resolved))
}

pub fn build_core_update(input: &CoreUpdateInput<'_>) -> CoreUpdateRows {
    // A release the daemon already runs (it just swapped to it) is not an update.
    let available_ref = input
        .available_ref
        .filter(|a| !same_release(a, input.resolved_ref));
    let updating = input.state == RunState::Updating || input.update_pending;
    let failure = input.update_error.or(input.recheck_error);
    let failed_run = match input.problem {
        Some(ArrowProblem::Failed { detail }) if !updating && failure.is_none() => {
            Some(detail.clone())
        }
        _ => None,
    };
    let note = if let Some(f) = failure {
        CoreUpdateNote::Failure(f.message.clone())
    } else if updating {
        CoreUpdateNote::Restarts
    } else if let Some(detail) = failed_run {
        CoreUpdateNote::FailedRun(detail)
    } else {
        CoreUpdateNote::None
    };
    CoreUpdateRows {
        installed: (!input.resolved_ref.is_empty()).then(|| input.resolved_ref.to_owned()),
        available: available_ref.map(str::to_owned),
        note,
        show_retry: failure.is_some_and(is_retryable),
        retry_is_update: input.update_error.is_some(),
        check_enabled: !(updating || input.recheck_pending),
        show_update: available_ref.is_some(),
        update_enabled: !updating,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> CoreUpdateInput<'static> {
        CoreUpdateInput {
            resolved_ref: "stable-26.5",
            available_ref: None,
            state: RunState::Ready,
            update_pending: false,
            recheck_pending: false,
            update_error: None,
            recheck_error: None,
            problem: None,
        }
    }

    fn fail(status: Option<u16>) -> ActionFailure {
        ActionFailure {
            status,
            message: "nope".into(),
        }
    }

    #[test]
    fn channels_start_loading_and_fail_closed() {
        let initial = CoreChannelsState::default();
        assert!(initial.loading && !initial.select_enabled());
        let failed = CoreChannelsState::loaded(Err(fail(Some(404))));
        assert!(
            failed.failed
                && failed.channels.is_empty()
                && !failed.loading
                && !failed.select_enabled()
        );
        assert!(!CoreChannelsState::loaded(Ok(vec![])).select_enabled());
    }

    #[test]
    fn only_transport_429_and_5xx_failures_are_retryable() {
        assert!(is_retryable(&fail(None)));
        assert!(is_retryable(&fail(Some(429))));
        assert!(is_retryable(&fail(Some(503))));
        assert!(!is_retryable(&fail(Some(422))));
        assert!(!is_retryable(&fail(Some(404))));
    }

    #[test]
    fn the_rows_show_installed_and_available_and_offer_update_only_when_something_is_ahead() {
        let rows = build_core_update(&input());
        assert_eq!(rows.installed.as_deref(), Some("stable-26.5"));
        assert_eq!(rows.available, None);
        assert!(!rows.show_update && rows.check_enabled);
        let ahead = CoreUpdateInput {
            available_ref: Some("stable-26.6"),
            ..input()
        };
        let rows = build_core_update(&ahead);
        assert!(rows.show_update && rows.update_enabled);
        let swapped = CoreUpdateInput {
            available_ref: Some("stable-26.5"),
            ..input()
        };
        let rows = build_core_update(&swapped);
        assert!(!rows.show_update && rows.available.is_none());
    }

    #[test]
    fn while_updating_the_controls_lock_and_the_note_says_it_restarts() {
        for i in [
            CoreUpdateInput {
                state: RunState::Updating,
                ..input()
            },
            CoreUpdateInput {
                update_pending: true,
                ..input()
            },
        ] {
            let rows = build_core_update(&i);
            assert_eq!(rows.note, CoreUpdateNote::Restarts);
            assert!(!rows.check_enabled && !rows.update_enabled);
        }
    }

    #[test]
    fn a_failure_wins_and_retry_reruns_what_failed() {
        let err = fail(Some(500));
        let rows = build_core_update(&CoreUpdateInput {
            update_error: Some(&err),
            ..input()
        });
        assert_eq!(rows.note, CoreUpdateNote::Failure("nope".into()));
        assert!(rows.show_retry && rows.retry_is_update);
        let rows = build_core_update(&CoreUpdateInput {
            recheck_error: Some(&err),
            ..input()
        });
        assert!(rows.show_retry && !rows.retry_is_update);
        let refused = fail(Some(422));
        assert!(
            !build_core_update(&CoreUpdateInput {
                update_error: Some(&refused),
                ..input()
            })
            .show_retry
        );
    }

    #[test]
    fn a_failed_run_is_reported_when_nothing_else_is_going_on() {
        let problem = ArrowProblem::Failed {
            detail: Some("exit 1".into()),
        };
        let rows = build_core_update(&CoreUpdateInput {
            problem: Some(&problem),
            ..input()
        });
        assert_eq!(rows.note, CoreUpdateNote::FailedRun(Some("exit 1".into())));
        let updating = CoreUpdateInput {
            problem: Some(&problem),
            state: RunState::Updating,
            ..input()
        };
        assert_eq!(build_core_update(&updating).note, CoreUpdateNote::Restarts);
    }
}
