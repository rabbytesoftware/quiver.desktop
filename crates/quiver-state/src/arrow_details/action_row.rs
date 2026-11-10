//! The header's action row (new design): 40 px buttons each with an attached info (ⓘ) button; the
//! action that is running becomes a [`ProgressControl`]; Settings and Methods are ghost icons at the
//! row's end.

use super::actions::{compute_actions, ArrowAction, ArrowActionKind, ArrowActionVariant};
use super::facts::{ArrowFacts, StepSpec};
use super::progress::{build_progress, verb_for, ProgressControl};
use super::status::RunState;
use crate::i18n::{en::arrow, en::design, Key};

/// What the ⓘ of a button previews.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ActionInfo {
    pub title: Key,
    pub steps: Vec<StepSpec>,
    pub uses_variables: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ActionButton {
    pub kind: ArrowActionKind,
    pub label: Key,
    pub variant: ArrowActionVariant,
    pub enabled: bool,
    /// In flight but without a progress control (Start, Add to Library, ...): shows its busy label.
    pub busy: bool,
    /// Absent when core exposes no step list (e.g. "Add to Library"): no ⓘ is drawn.
    pub info: Option<ActionInfo>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ActionSlot {
    Button(ActionButton),
    Progress(ProgressControl),
}

#[derive(Clone, PartialEq, Debug)]
pub struct ActionRow {
    pub slots: Vec<ActionSlot>,
    /// Settings ghost icon: only when the arrow declares variables.
    pub show_settings: bool,
    /// Methods ghost icon (inspect only): always, the arrow has at least its lifecycle.
    pub show_methods: bool,
}

impl ActionRow {
    pub fn progress(&self) -> Option<&ProgressControl> {
        self.slots.iter().find_map(|slot| match slot {
            ActionSlot::Progress(p) => Some(p),
            ActionSlot::Button(_) => None,
        })
    }

    pub fn button(
        &self,
        kind: ArrowActionKind,
    ) -> Option<&ActionButton> {
        self.slots.iter().find_map(|slot| match slot {
            ActionSlot::Button(b) if b.kind == kind => Some(b),
            _ => None,
        })
    }
}

fn stopping_verb(state: RunState) -> Option<Key> {
    match state {
        RunState::Draining => Some(design::VERB_DRAINING),
        RunState::Stopping => Some(design::VERB_STOPPING),
        _ => None,
    }
}

fn button(
    action: &ArrowAction,
    busy: bool,
    switching: bool,
) -> ActionButton {
    let label = if busy {
        action.busy_label.unwrap_or(action.label)
    } else {
        action.label
    };
    ActionButton {
        kind: action.kind,
        label,
        variant: action.variant,
        enabled: !(busy || action.force_busy || action.force_disabled || switching),
        busy,
        info: (!action.steps.is_empty()).then(|| ActionInfo {
            title: action.label,
            steps: action.steps.clone(),
            uses_variables: action.uses_variables.clone(),
        }),
    }
}

/// Builds the row. `pending` is the action whose request is in flight before core's `active_run`
/// catches up over the live feed; `switching` is true while a selector switch drives install/uninstall
/// on this identity (every action is then disabled).
pub fn build_action_row(
    facts: &ArrowFacts,
    pending: Option<ArrowActionKind>,
    switching: bool,
) -> ActionRow {
    let mut slots = Vec::new();
    for action in compute_actions(facts) {
        let in_flight = action.force_busy || pending == Some(action.kind);
        let verb = in_flight
            .then(|| verb_for(action.kind, stopping_verb(facts.state)))
            .flatten();
        match verb {
            Some(verb) => slots.push(ActionSlot::Progress(build_progress(
                verb,
                facts.active_run.as_ref(),
                &action.steps,
            ))),
            None => slots.push(ActionSlot::Button(button(&action, in_flight, switching))),
        }
    }
    ActionRow {
        slots,
        show_settings: !facts.variable_names.is_empty(),
        show_methods: facts.lifecycle.is_some() || !facts.methods.is_empty(),
    }
}

/// Label of the ⓘ trigger (shared across buttons).
pub const INFO_LABEL: Key = arrow::ACTION_INFO;
