//! Inputs the view layer fills from the store. Small on purpose: the decision logic of this
//! feature depends on these shapes, not on the data layer's DTOs.

use super::status::RunState;

/// One declared step of a lifecycle action or method.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StepSpec {
    pub title: String,
    /// `run`, `fetch`, `extract`, `portable`, `signal`, `dependencies`, `expose`, `unexpose` (a bare
    /// string off the wire: an unknown kind is allowed).
    pub step_type: String,
    /// The `run` step's command, when it has one (used to tell "Open" from "Start").
    pub command: Option<String>,
    /// The raw step as the manifest writes it (YAML), shown by the step inspector.
    pub code: String,
}

/// The five reserved lifecycle verbs of the platform's target.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct LifecycleFacts {
    pub install: Vec<StepSpec>,
    pub update: Vec<StepSpec>,
    pub execute: Vec<StepSpec>,
    pub stop: Vec<StepSpec>,
    pub uninstall: Vec<StepSpec>,
}

/// A custom method of the manifest (e.g. `backup`, `rcon`).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MethodFacts {
    pub name: String,
    pub description: String,
    pub steps: Vec<StepSpec>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Runtime progress of one step of the active run.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RunStepFacts {
    pub title: String,
    pub step_type: String,
    pub status: StepStatus,
    pub error: Option<String>,
    pub note: Option<String>,
}

/// The run core reports for the arrow right now (`active_run`).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ActiveRunFacts {
    pub method: String,
    pub steps: Vec<RunStepFacts>,
}

/// Everything the header and action row need to know about one arrow.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ArrowFacts {
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub license: String,
    /// The ref the row resolved to (what is installed, or would be).
    pub resolved_ref: String,
    pub state: RunState,
    pub user_installed: bool,
    /// What the last version check found ahead of `resolved_ref`.
    pub available_ref: Option<String>,
    /// Names of every variable the manifest declares.
    pub variable_names: Vec<String>,
    /// Lifecycle of the target matching this machine (or the first target); `None` without targets.
    pub lifecycle: Option<LifecycleFacts>,
    pub methods: Vec<MethodFacts>,
    /// A target with exactly this machine's platform exists.
    pub platform_supported: bool,
    /// Platforms the arrow publishes a build for, for the "Runs on" line.
    pub platforms: Vec<String>,
    pub active_run: Option<ActiveRunFacts>,
    pub inferred: bool,
}

impl ArrowFacts {
    pub fn update_ahead(&self) -> bool {
        self.available_ref.is_some()
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub fn step(title: &str, step_type: &str) -> StepSpec {
        StepSpec {
            title: title.into(),
            step_type: step_type.into(),
            command: None,
            code: format!("{step_type}: {title}"),
        }
    }

    pub fn lifecycle() -> LifecycleFacts {
        LifecycleFacts {
            install: vec![step("Download", "fetch"), step("Extract", "extract")],
            update: vec![step("Download", "fetch")],
            execute: vec![step("Run", "run")],
            stop: vec![step("Signal", "signal")],
            uninstall: vec![step("Remove", "run")],
        }
    }

    pub fn installed(state: RunState) -> ArrowFacts {
        ArrowFacts {
            namespace: "github.com/rabbyte/caddy@stable".into(),
            name: "Caddy".into(),
            user_installed: true,
            state,
            lifecycle: Some(lifecycle()),
            platform_supported: true,
            ..ArrowFacts::default()
        }
    }

    pub fn discovered() -> ArrowFacts {
        ArrowFacts {
            user_installed: false,
            state: RunState::Absent,
            ..installed(RunState::Absent)
        }
    }
}
