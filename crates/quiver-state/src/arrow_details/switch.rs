//! Moving what a library entry follows (port of `switch-selector.ts`, `selector-switch-store.ts` and the
//! pure part of `use-selector-switch.ts`).
//!
//! A catalog identity's selector never changes, so there is no in-place switch. The new identity is
//! registered FIRST: rows coexist, so registering changes nothing about the old one, and a selector core
//! cannot resolve fails there with nothing lost. Only then is the old row uninstalled (when it was
//! installed) and forgotten, and the new one installed in its place. Strictly sequential.

use super::status::RunState;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwitchStep {
    Register,
    Uninstall,
    Remove,
    Install,
}

impl SwitchStep {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Uninstall => "uninstall",
            Self::Remove => "remove",
            Self::Install => "install",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwitchPlan {
    pub from: String,
    pub to: String,
    /// Whether `from` has anything on disk: only then is there something to uninstall and put back.
    pub installed: bool,
}

impl SwitchPlan {
    /// The steps to run, in order. A failure names the step it stopped at: every step before it has
    /// happened, none after it has.
    pub fn steps(&self) -> Vec<SwitchStep> {
        let mut steps = vec![SwitchStep::Register];
        if self.installed {
            steps.push(SwitchStep::Uninstall);
        }
        steps.push(SwitchStep::Remove);
        if self.installed {
            steps.push(SwitchStep::Install);
        }
        steps
    }
}

/// Only a row at rest can be uninstalled and forgotten: anything in flight, running or detached has
/// to settle first.
pub fn can_switch(state: RunState) -> bool {
    matches!(
        state,
        RunState::Absent | RunState::Ready | RunState::Outdated
    )
}

/// A switch that stopped after the new identity was registered.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwitchFailure {
    pub step: SwitchStep,
    pub from: String,
    pub to: String,
    pub reason: String,
}

/// The switch in flight and how the last one ended. It outlives the page a switch started on: the page
/// moves to the new identity as soon as it is registered, so what happens after cannot live in page state.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SelectorSwitchState {
    pub active: Option<(String, String)>,
    pub failure: Option<SwitchFailure>,
}

impl SelectorSwitchState {
    pub fn begin(
        &mut self,
        from: &str,
        to: &str,
    ) {
        self.active = Some((from.to_owned(), to.to_owned()));
        self.failure = None;
    }

    pub fn end(
        &mut self,
        failure: Option<SwitchFailure>,
    ) {
        self.active = None;
        self.failure = failure;
    }

    pub fn dismiss(&mut self) {
        self.failure = None;
    }

    /// A switch from or to this identity is in flight.
    pub fn involves_active(
        &self,
        namespace: &str,
    ) -> bool {
        self.active
            .as_ref()
            .is_some_and(|(from, to)| from == namespace || to == namespace)
    }

    /// The failure, scoped to the two identities it is about (its sentences only make sense there).
    pub fn failure_for(
        &self,
        namespace: &str,
    ) -> Option<&SwitchFailure> {
        self.failure
            .as_ref()
            .filter(|f| f.from == namespace || f.to == namespace)
    }
}

/// Settles once the live store reports a namespace not installed: done at `absent`/`removed` (including
/// when already there, e.g. after an uninstall core answered as a no-op); an error when the row settles
/// anywhere else after having started uninstalling. The caller subscribes BEFORE requesting the
/// uninstall and feeds every state it sees to [`UninstallWatch::inspect`], so a fast transition cannot
/// slip past. The timeout is the caller's.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UninstallWatch {
    namespace: String,
    started: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UninstallProgress {
    Waiting,
    Done,
    Failed(String),
}

impl UninstallWatch {
    pub fn new(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_owned(),
            started: false,
        }
    }

    pub fn inspect(
        &mut self,
        state: Option<RunState>,
    ) -> UninstallProgress {
        let Some(state) = state else {
            return UninstallProgress::Waiting;
        };
        match state {
            RunState::Absent | RunState::Removed => UninstallProgress::Done,
            RunState::Uninstalling => {
                self.started = true;
                UninstallProgress::Waiting
            }
            other if self.started => UninstallProgress::Failed(format!(
                "{} did not finish uninstalling (it is {})",
                self.namespace,
                state_name(other)
            )),
            _ => UninstallProgress::Waiting,
        }
    }
}

fn state_name(state: RunState) -> &'static str {
    match state {
        RunState::Absent => "absent",
        RunState::Installing => "installing",
        RunState::Updating => "updating",
        RunState::Ready => "ready",
        RunState::Running => "running",
        RunState::Stopping => "stopping",
        RunState::Draining => "draining",
        RunState::Detached => "detached",
        RunState::Uninstalling => "uninstalling",
        RunState::Removed => "removed",
        RunState::Outdated => "outdated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(installed: bool) -> SwitchPlan {
        SwitchPlan {
            from: "a@stable".into(),
            to: "a@v1".into(),
            installed,
        }
    }

    #[test]
    fn an_installed_row_registers_first_then_uninstalls_forgets_and_reinstalls() {
        use SwitchStep::*;
        assert_eq!(plan(true).steps(), [Register, Uninstall, Remove, Install]);
    }

    #[test]
    fn a_row_with_nothing_on_disk_only_registers_and_forgets() {
        use SwitchStep::*;
        assert_eq!(plan(false).steps(), [Register, Remove]);
    }

    #[test]
    fn only_a_row_at_rest_can_switch() {
        use RunState::*;
        for ok in [Absent, Ready, Outdated] {
            assert!(can_switch(ok));
        }
        for busy in [
            Installing,
            Updating,
            Running,
            Stopping,
            Draining,
            Detached,
            Uninstalling,
            Removed,
        ] {
            assert!(!can_switch(busy), "{busy:?}");
        }
    }

    #[test]
    fn the_store_scopes_a_failure_to_its_two_identities() {
        let mut store = SelectorSwitchState::default();
        store.begin("a@stable", "a@v1");
        assert!(store.involves_active("a@v1"));
        assert!(!store.involves_active("b"));
        let failure = SwitchFailure {
            step: SwitchStep::Install,
            from: "a@stable".into(),
            to: "a@v1".into(),
            reason: "boom".into(),
        };
        store.end(Some(failure));
        assert!(store.active.is_none());
        assert!(store.failure_for("a@stable").is_some());
        assert!(store.failure_for("other").is_none());
        store.dismiss();
        assert!(store.failure.is_none());
    }

    #[test]
    fn beginning_a_switch_clears_the_previous_failure() {
        let mut store = SelectorSwitchState::default();
        store.end(Some(SwitchFailure {
            step: SwitchStep::Remove,
            from: "x".into(),
            to: "y".into(),
            reason: String::new(),
        }));
        store.begin("p", "q");
        assert!(store.failure.is_none());
    }

    #[test]
    fn the_uninstall_watch_resolves_at_absent_or_removed_even_if_already_there() {
        assert_eq!(
            UninstallWatch::new("a").inspect(Some(RunState::Absent)),
            UninstallProgress::Done
        );
        assert_eq!(
            UninstallWatch::new("a").inspect(Some(RunState::Removed)),
            UninstallProgress::Done
        );
    }

    #[test]
    fn the_uninstall_watch_fails_when_the_row_settles_elsewhere_after_starting() {
        let mut watch = UninstallWatch::new("a");
        assert_eq!(watch.inspect(None), UninstallProgress::Waiting);
        assert_eq!(
            watch.inspect(Some(RunState::Ready)),
            UninstallProgress::Waiting
        );
        assert_eq!(
            watch.inspect(Some(RunState::Uninstalling)),
            UninstallProgress::Waiting
        );
        assert_eq!(
            watch.inspect(Some(RunState::Ready)),
            UninstallProgress::Failed("a did not finish uninstalling (it is ready)".into())
        );
    }
}
