//! The state machine behind the action row (port of the decision logic of `use-hero-actions.ts`):
//! what is pending, which error modal is open, the "up to date" note, the platform gate on adding and
//! restart's client-side stop-then-execute sequencing. The calls to core are the view layer's; every
//! decision it needs is here.

use super::actions::ArrowActionKind;
use super::release_variables::{is_self_arrow, release_error_message_key, ReleaseFailure};
use super::status::RunState;
use crate::i18n::Key;

/// How long "already up to date" stays on screen.
pub const UP_TO_DATE_MS: u64 = 4000;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReleaseError {
    pub message: Key,
    pub detail: String,
}

/// A mutation answered with an HTTP error.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ActionFailure {
    /// HTTP status when the failure came from Core's API.
    pub status: Option<u16>,
    pub message: String,
}

/// What the update call answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UpdateOutcome {
    Started,
    /// A 200: no run starts and no runtime event follows, so there is nothing to wait for.
    Current,
}

/// A state violation (422) or conflict (409) on update is a race with another update (one underway, or
/// the last one's commit still pending) which a retry a moment later settles. Every other failure is final.
pub fn is_retryable_update_error(failure: &ActionFailure) -> bool {
    matches!(failure.status, Some(409 | 422))
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct InvokeState {
    /// The action in flight, or `None`: the matching slot disables / becomes the progress control.
    pub pending: Option<ArrowActionKind>,
    pub release_error: Option<ReleaseError>,
    pub action_error: Option<String>,
    pub retry: Option<ArrowActionKind>,
    pub up_to_date: bool,
    /// The platform the add was refused for, to confirm with the person.
    pub platform_warning: Option<String>,
    restarting: bool,
}

/// Which kinds fetch a release asset that `ARROW.md` cannot name (Quiver's own row only).
pub fn needs_release(kind: ArrowActionKind, namespace: &str) -> bool {
    matches!(
        kind,
        ArrowActionKind::Install | ArrowActionKind::Reinstall | ArrowActionKind::Update
    ) && is_self_arrow(namespace)
}

/// Adding an arrow that is not in the library asks about its platform first, on a FRESH authoritative
/// read (never the reactive value: a click before it resolved would silently skip the warning).
pub fn needs_platform_check(
    kind: ArrowActionKind,
    user_installed: bool,
    skip_warning: bool,
) -> bool {
    (kind == ArrowActionKind::AddToLibrary || !user_installed) && !skip_warning
}

impl InvokeState {
    /// Set before any async work: the disabled-while-busy render is what stops a second click from
    /// re-entering during the platform check.
    pub fn begin(&mut self, kind: ArrowActionKind) {
        self.pending = Some(kind);
        self.up_to_date = false;
        self.retry = None;
    }

    pub fn platform_refused(&mut self, real_platform: &str) {
        self.pending = None;
        self.platform_warning = Some(real_platform.to_owned());
    }

    pub fn dismiss_platform_warning(&mut self) {
        self.platform_warning = None;
    }

    /// Resolving Quiver's own release asset failed before any call to core. Returns true when the page's
    /// `available` was stale (`nothing_ahead`) and the detail should be re-read.
    pub fn release_failed(&mut self, failure: &ReleaseFailure, kind_wire: &str) -> bool {
        self.release_error = Some(ReleaseError {
            message: release_error_message_key(kind_wire),
            detail: failure.detail.clone(),
        });
        self.pending = None;
        kind_wire == "nothing_ahead"
    }

    pub fn dismiss_release_error(&mut self) {
        self.release_error = None;
    }

    /// The call succeeded. Restart keeps `pending` through both legs: cleared once `execute` resolves.
    pub fn succeeded(&mut self, kind: ArrowActionKind, update: Option<UpdateOutcome>) {
        if kind == ArrowActionKind::Restart {
            self.restarting = true;
            return;
        }
        if kind == ArrowActionKind::Update && update == Some(UpdateOutcome::Current) {
            self.up_to_date = true;
        }
        self.pending = None;
    }

    /// A mutation rejected, carrying the backend's own precise reason.
    pub fn failed(&mut self, kind: ArrowActionKind, failure: &ActionFailure) {
        self.restarting = false;
        self.pending = None;
        self.action_error = Some(failure.message.clone());
        if kind == ArrowActionKind::Update && is_retryable_update_error(failure) {
            self.retry = Some(ArrowActionKind::Update);
        }
    }

    pub fn dismiss_action_error(&mut self) {
        self.action_error = None;
        self.retry = None;
    }

    pub fn expire_up_to_date(&mut self) {
        self.up_to_date = false;
    }

    /// Restart's second leg: core only accepts `execute` once the arrow has genuinely reached `ready`
    /// (not merely once the `stop` request was accepted), so the view calls this on every live state
    /// change and runs `execute` when it returns true.
    pub fn restart_ready_for_execute(&mut self, state: RunState) -> bool {
        if !self.restarting || state != RunState::Ready {
            return false;
        }
        self.restarting = false;
        true
    }

    /// The second leg finished (either way): the restart is over.
    pub fn restart_finished(&mut self, failure: Option<&ActionFailure>) {
        if let Some(failure) = failure {
            self.action_error = Some(failure.message.clone());
        }
        self.pending = None;
    }
}

/// A toast ("Caddy installed") when an install settles into `ready`.
pub fn installed_toast_due(previous: RunState, now: RunState) -> bool {
    previous == RunState::Installing && now == RunState::Ready
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_details::release_variables::ReleaseResolveErrorKind;
    use ArrowActionKind as K;

    fn failure(status: Option<u16>) -> ActionFailure {
        ActionFailure {
            status,
            message: "nope".into(),
        }
    }

    #[test]
    fn only_409_and_422_on_update_are_worth_retrying() {
        assert!(is_retryable_update_error(&failure(Some(409))));
        assert!(is_retryable_update_error(&failure(Some(422))));
        assert!(!is_retryable_update_error(&failure(Some(500))));
        assert!(!is_retryable_update_error(&failure(None)));
    }

    #[test]
    fn a_failed_update_race_offers_a_retry_other_failures_do_not() {
        let mut state = InvokeState::default();
        state.begin(K::Update);
        state.failed(K::Update, &failure(Some(409)));
        assert_eq!(state.retry, Some(K::Update));
        assert_eq!(state.action_error.as_deref(), Some("nope"));
        assert_eq!(state.pending, None);
        state.dismiss_action_error();
        assert_eq!((state.retry, state.action_error.as_deref()), (None, None));
        state.begin(K::Stop);
        state.failed(K::Stop, &failure(Some(409)));
        assert_eq!(state.retry, None);
    }

    #[test]
    fn begin_clears_the_previous_attempts_leftovers() {
        let mut state = InvokeState {
            up_to_date: true,
            retry: Some(K::Update),
            ..InvokeState::default()
        };
        state.begin(K::Install);
        assert_eq!(
            (state.pending, state.up_to_date, state.retry),
            (Some(K::Install), false, None)
        );
    }

    #[test]
    fn an_update_core_answers_as_current_shows_the_note_and_settles() {
        let mut state = InvokeState::default();
        state.begin(K::Update);
        state.succeeded(K::Update, Some(UpdateOutcome::Current));
        assert!(state.up_to_date);
        assert_eq!(state.pending, None);
        state.expire_up_to_date();
        assert!(!state.up_to_date);
        state.begin(K::Update);
        state.succeeded(K::Update, Some(UpdateOutcome::Started));
        assert!(!state.up_to_date);
    }

    #[test]
    fn restart_stays_pending_until_the_arrow_is_ready_again() {
        let mut state = InvokeState::default();
        state.begin(K::Restart);
        state.succeeded(K::Restart, None);
        assert_eq!(state.pending, Some(K::Restart));
        assert!(!state.restart_ready_for_execute(RunState::Stopping));
        assert!(state.restart_ready_for_execute(RunState::Ready));
        assert!(
            !state.restart_ready_for_execute(RunState::Ready),
            "the second leg runs once"
        );
        state.restart_finished(None);
        assert_eq!(state.pending, None);
    }

    #[test]
    fn a_failure_between_the_legs_cancels_the_restart() {
        let mut state = InvokeState::default();
        state.begin(K::Restart);
        state.succeeded(K::Restart, None);
        state.failed(K::Restart, &failure(None));
        assert!(!state.restart_ready_for_execute(RunState::Ready));
    }

    #[test]
    fn the_second_legs_failure_is_reported() {
        let mut state = InvokeState::default();
        state.begin(K::Restart);
        state.restart_finished(Some(&failure(None)));
        assert_eq!(state.action_error.as_deref(), Some("nope"));
        assert_eq!(state.pending, None);
    }

    #[test]
    fn adding_asks_about_the_platform_unless_told_not_to() {
        assert!(needs_platform_check(K::AddToLibrary, true, false));
        assert!(needs_platform_check(K::Install, false, false));
        assert!(!needs_platform_check(K::Install, true, false));
        assert!(!needs_platform_check(K::AddToLibrary, false, true));
        let mut state = InvokeState::default();
        state.begin(K::AddToLibrary);
        state.platform_refused("linux/arm64");
        assert_eq!(state.platform_warning.as_deref(), Some("linux/arm64"));
        assert_eq!(state.pending, None);
        state.dismiss_platform_warning();
        assert_eq!(state.platform_warning, None);
    }

    #[test]
    fn only_quivers_own_install_and_update_need_release_variables() {
        let own = "github.com/rabbytesoftware/quiver.desktop@stable";
        assert!(needs_release(K::Install, own));
        assert!(needs_release(K::Reinstall, own));
        assert!(needs_release(K::Update, own));
        assert!(!needs_release(K::Stop, own));
        assert!(!needs_release(K::Install, "github.com/rabbyte/caddy"));
    }

    #[test]
    fn a_release_failure_opens_its_sentence_and_flags_a_stale_available() {
        let mut state = InvokeState::default();
        state.begin(K::Update);
        let fail = ReleaseFailure {
            kind: ReleaseResolveErrorKind::NothingAhead,
            detail: "d".into(),
        };
        assert!(state.release_failed(&fail, "nothing_ahead"));
        assert_eq!(state.pending, None);
        let error = state.release_error.clone().unwrap();
        assert_eq!(error.message, crate::i18n::en::arrow::RELEASE_NOTHING_AHEAD);
        assert!(!state.release_failed(&fail, "offline"));
        state.dismiss_release_error();
        assert!(state.release_error.is_none());
    }

    #[test]
    fn an_install_that_lands_on_ready_toasts_once() {
        assert!(installed_toast_due(RunState::Installing, RunState::Ready));
        assert!(!installed_toast_due(RunState::Ready, RunState::Ready));
        assert!(!installed_toast_due(RunState::Updating, RunState::Ready));
    }
}
