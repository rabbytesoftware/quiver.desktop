//! The running action as one control (new design): verb on line 1, step and count on line 2, a soft
//! fill as the progress and a Steps popover. No cancel: Core cannot cancel an install or update.

use super::actions::ArrowActionKind;
use super::facts::{ActiveRunFacts, RunStepFacts, StepSpec, StepStatus};
use crate::i18n::{en::design, Key, Params, Translator};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StepState {
    Done,
    Running,
    Pending,
    /// The step failed. Core ends the run there, so the control is replaced by the failure state.
    Failed,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProgressStep {
    pub state: StepState,
    pub title: String,
    /// What the step does in plain words (translated by the view; the `arrow.stepPlain.*` keys).
    pub plain_text: Key,
    /// The bare step code, shown for the running step and in the step's own inspector.
    pub code: String,
    pub step_type: String,
    pub error: Option<String>,
    pub note: Option<String>,
}

/// "Verify checksum · 2 of 4"
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StepLabel {
    pub title: String,
    pub index: usize,
    pub total: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProgressControl {
    pub verb: Key,
    /// `None` until core reports the run's steps (the request is accepted but nothing has started).
    pub step_label: Option<StepLabel>,
    /// 1-based number of the current step; 0 before any step is known.
    pub index: usize,
    pub total: usize,
    /// 0..=1 fill, `None` while indeterminate (no steps known yet).
    pub fraction: Option<f32>,
    pub steps: Vec<ProgressStep>,
}

impl ProgressControl {
    /// Line 2 of the control, e.g. "Verify checksum · 2 of 4"; "Starting…" before the steps are known.
    pub fn step_line(&self, tr: &Translator) -> String {
        match &self.step_label {
            Some(label) => tr.t_with(
                design::STEP_OF,
                &Params::new()
                    .with("title", label.title.as_str())
                    .with("index", label.index)
                    .with("total", label.total),
            ),
            None => tr.t(design::STARTING),
        }
    }
}

/// The verb for the action in flight. `None` for actions that have no progress control (Start,
/// Add to Library, ...): those keep their button, busy.
pub fn verb_for(kind: ArrowActionKind, stopping_state: Option<Key>) -> Option<Key> {
    match kind {
        ArrowActionKind::Install | ArrowActionKind::Reinstall => Some(design::VERB_INSTALLING),
        ArrowActionKind::Update => Some(design::VERB_UPDATING),
        ArrowActionKind::Uninstall => Some(design::VERB_UNINSTALLING),
        ArrowActionKind::Stop | ArrowActionKind::Restart => {
            Some(stopping_state.unwrap_or(design::VERB_STOPPING))
        }
        _ => None,
    }
}

/// Plain-words description key for a step type (unknown kinds get a generic sentence).
pub fn plain_text_for(step_type: &str) -> Key {
    match step_type {
        "run" => design::PLAIN_RUN,
        "fetch" => design::PLAIN_FETCH,
        "extract" => design::PLAIN_EXTRACT,
        "portable" => design::PLAIN_PORTABLE,
        "signal" => design::PLAIN_SIGNAL,
        "dependencies" => design::PLAIN_DEPENDENCIES,
        "expose" => design::PLAIN_EXPOSE,
        "unexpose" => design::PLAIN_UNEXPOSE,
        _ => design::PLAIN_OTHER,
    }
}

fn step_state(status: StepStatus) -> StepState {
    match status {
        StepStatus::Completed => StepState::Done,
        StepStatus::Running => StepState::Running,
        StepStatus::Pending => StepState::Pending,
        StepStatus::Failed => StepState::Failed,
    }
}

/// `declared` supplies the code of each step by position when the run does not carry it.
fn progress_step(run: &RunStepFacts, declared: Option<&StepSpec>) -> ProgressStep {
    ProgressStep {
        state: step_state(run.status),
        title: run.title.clone(),
        plain_text: plain_text_for(&run.step_type),
        code: declared.map(|s| s.code.clone()).unwrap_or_default(),
        step_type: run.step_type.clone(),
        error: run.error.clone(),
        note: run.note.clone(),
    }
}

/// Builds the control from the active run. `declared` are the steps the action's method declares (for
/// the step code); the fill counts a running step as half done so a long first step does not look stuck.
pub fn build_progress(
    verb: Key,
    run: Option<&ActiveRunFacts>,
    declared: &[StepSpec],
) -> ProgressControl {
    let Some(run) = run.filter(|r| !r.steps.is_empty()) else {
        return ProgressControl {
            verb,
            step_label: None,
            index: 0,
            total: 0,
            fraction: None,
            steps: Vec::new(),
        };
    };
    let total = run.steps.len();
    let steps: Vec<ProgressStep> = run
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| progress_step(step, declared.get(i)))
        .collect();
    let current = run
        .steps
        .iter()
        .position(|s| s.status == StepStatus::Running)
        .or_else(|| {
            run.steps
                .iter()
                .position(|s| s.status == StepStatus::Pending)
        })
        .unwrap_or(total - 1);
    let done = run
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Completed)
        .count();
    let running = run
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Running)
        .count();
    let fraction = ((done as f32) + 0.5 * running as f32) / total as f32;
    ProgressControl {
        verb,
        step_label: Some(StepLabel {
            title: run.steps[current].title.clone(),
            index: current + 1,
            total,
        }),
        index: current + 1,
        total,
        fraction: Some(fraction.clamp(0.0, 1.0)),
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(statuses: &[StepStatus]) -> ActiveRunFacts {
        ActiveRunFacts {
            method: "update".into(),
            steps: statuses
                .iter()
                .enumerate()
                .map(|(i, status)| RunStepFacts {
                    title: ["Download", "Verify checksum", "Extract", "Restart"][i % 4].into(),
                    step_type: "fetch".into(),
                    status: *status,
                    error: None,
                    note: None,
                })
                .collect(),
        }
    }

    #[test]
    fn line_two_names_the_running_step_and_its_count() {
        use StepStatus::*;
        let control = build_progress(
            design::VERB_UPDATING,
            Some(&run(&[Completed, Running, Pending, Pending])),
            &[],
        );
        assert_eq!(
            control.step_line(&Translator::default()),
            "Verify checksum · 2 of 4"
        );
        assert_eq!((control.index, control.total), (2, 4));
        assert_eq!(control.fraction, Some(0.375));
        let states: Vec<_> = control.steps.iter().map(|s| s.state).collect();
        assert_eq!(
            states,
            [
                StepState::Done,
                StepState::Running,
                StepState::Pending,
                StepState::Pending
            ]
        );
    }

    #[test]
    fn between_steps_the_next_pending_one_is_current() {
        use StepStatus::*;
        let control = build_progress(
            design::VERB_INSTALLING,
            Some(&run(&[Completed, Pending])),
            &[],
        );
        assert_eq!(control.step_label.as_ref().map(|l| l.index), Some(2));
        assert_eq!(control.fraction, Some(0.5));
    }

    #[test]
    fn a_finished_run_stays_on_the_last_step_at_full() {
        use StepStatus::*;
        let control = build_progress(
            design::VERB_INSTALLING,
            Some(&run(&[Completed, Completed])),
            &[],
        );
        assert_eq!(control.index, 2);
        assert_eq!(control.fraction, Some(1.0));
    }

    #[test]
    fn before_core_reports_steps_it_is_indeterminate() {
        let control = build_progress(design::VERB_UPDATING, None, &[]);
        assert_eq!(control.fraction, None);
        assert!(control.steps.is_empty());
        assert_eq!(control.step_line(&Translator::default()), "Starting…");
        let empty = ActiveRunFacts::default();
        assert_eq!(
            build_progress(design::VERB_UPDATING, Some(&empty), &[]).total,
            0
        );
    }

    #[test]
    fn step_code_comes_from_the_declared_steps_by_position() {
        use StepStatus::*;
        let declared = [
            StepSpec {
                code: "fetch: url".into(),
                ..StepSpec::default()
            },
            StepSpec {
                code: "verify: sha".into(),
                ..StepSpec::default()
            },
        ];
        let control = build_progress(
            design::VERB_UPDATING,
            Some(&run(&[Completed, Running])),
            &declared,
        );
        assert_eq!(control.steps[1].code, "verify: sha");
        assert_eq!(control.steps[0].plain_text, design::PLAIN_FETCH);
    }

    #[test]
    fn only_actions_with_a_run_have_a_verb() {
        assert_eq!(
            verb_for(ArrowActionKind::Update, None),
            Some(design::VERB_UPDATING)
        );
        assert_eq!(
            verb_for(ArrowActionKind::Reinstall, None),
            Some(design::VERB_INSTALLING)
        );
        assert_eq!(
            verb_for(ArrowActionKind::Stop, Some(design::VERB_DRAINING)),
            Some(design::VERB_DRAINING)
        );
        assert_eq!(verb_for(ArrowActionKind::Execute, None), None);
        assert_eq!(verb_for(ArrowActionKind::AddToLibrary, None), None);
    }

    #[test]
    fn unknown_step_types_get_a_generic_sentence() {
        assert_eq!(plain_text_for("teleport"), design::PLAIN_OTHER);
        assert_eq!(plain_text_for("signal"), design::PLAIN_SIGNAL);
    }
}
