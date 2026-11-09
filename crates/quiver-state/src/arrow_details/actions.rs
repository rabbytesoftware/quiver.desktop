//! Which lifecycle actions an arrow offers in each state (port of `lib/actions.ts`).
//!
//! Verified against quiver.core's real gating: `execute` (the universal "Start") is hard-gated to
//! `ready` only, for every arrow, with no manifest override. Never render it enabled outside `ready`.

use super::facts::{ArrowFacts, LifecycleFacts, StepSpec};
use super::release_variables::{is_self_arrow, RELEASE_VARIABLE_NAMES};
use super::status::RunState;
use crate::i18n::{en::arrow, Key};

/// What an action invokes. `Execute` is core's one universal "go" (`POST .../execute`); `Restart` is a
/// client-side stop-then-execute sequence, not a single core call.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ArrowActionKind {
    AddToLibrary,
    Install,
    RemoveFromLibrary,
    Execute,
    Uninstall,
    Update,
    Stop,
    Restart,
    Reinstall,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrowActionVariant {
    Default,
    Outline,
    Destructive,
    DestructiveOutline,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ArrowAction {
    pub kind: ArrowActionKind,
    pub label: Key,
    /// Set only when `force_busy`: the label shown while in flight.
    pub busy_label: Option<Key>,
    pub variant: ArrowActionVariant,
    /// What the info trigger previews. Empty when core exposes no step list (e.g. "Add to Library").
    pub steps: Vec<StepSpec>,
    /// Variable names this call's `variables` body carries (core has no per-action scoping: all or none).
    pub uses_variables: Vec<String>,
    /// This is the action currently running: shows `busy_label`, its info stays clickable.
    pub force_busy: bool,
    /// Not runnable from the current state: plainly greyed, info stays clickable.
    pub force_disabled: bool,
}

impl ArrowAction {
    fn new(kind: ArrowActionKind, label: Key, variant: ArrowActionVariant) -> Self {
        Self {
            kind,
            label,
            busy_label: None,
            variant,
            steps: Vec::new(),
            uses_variables: Vec::new(),
            force_busy: false,
            force_disabled: false,
        }
    }

    fn steps(mut self, steps: &[StepSpec]) -> Self {
        self.steps = steps.to_vec();
        self
    }

    fn with_variables(mut self, names: &[String]) -> Self {
        self.uses_variables = names.to_vec();
        self
    }

    fn busy(mut self, label: Key) -> Self {
        self.force_busy = true;
        self.busy_label = Some(label);
        self
    }

    fn disabled(mut self, disabled: bool) -> Self {
        self.force_disabled = disabled;
        self
    }
}

/// The variables an arrow's configure form asks for: every declared one, except on Quiver's own row,
/// where the release URL and checksum are resolved at click time and overwritten (asking a person to
/// paste a download URL and a sha256 would be asking for something the app is about to replace).
fn all_variable_names(facts: &ArrowFacts) -> Vec<String> {
    if !is_self_arrow(&facts.namespace) {
        return facts.variable_names.clone();
    }
    facts
        .variable_names
        .iter()
        .filter(|name| !RELEASE_VARIABLE_NAMES.contains(&name.as_str()))
        .cloned()
        .collect()
}

/// "Open" for an arrow whose execute launches its exposed desktop app (core derives that step from
/// `expose.desktop` and runs `${ARROW_APP}`, which a service never does); everything else is "Start".
pub fn launch_label(lifecycle: Option<&LifecycleFacts>) -> Key {
    let launches_app = lifecycle.is_some_and(|l| {
        l.execute.iter().any(|step| {
            step.step_type == "run"
                && step
                    .command
                    .as_deref()
                    .is_some_and(|c| c.contains("ARROW_APP"))
        })
    });
    if launches_app {
        arrow::ACTION_OPEN
    } else {
        arrow::ACTION_START
    }
}

struct Ctx<'a> {
    lifecycle: Option<&'a LifecycleFacts>,
    config_vars: Vec<String>,
}

impl Ctx<'_> {
    fn has_execute(&self) -> bool {
        self.lifecycle.is_some_and(|l| !l.execute.is_empty())
    }

    fn has_install(&self) -> bool {
        self.lifecycle.is_some_and(|l| !l.install.is_empty())
    }

    fn pick(&self, f: impl Fn(&LifecycleFacts) -> &Vec<StepSpec>) -> Vec<StepSpec> {
        self.lifecycle.map(|l| f(l).clone()).unwrap_or_default()
    }

    fn execute(&self, disabled: bool, variant: ArrowActionVariant) -> ArrowAction {
        ArrowAction::new(
            ArrowActionKind::Execute,
            launch_label(self.lifecycle),
            variant,
        )
        .steps(&self.pick(|l| &l.execute))
        .with_variables(&self.config_vars)
        .disabled(disabled)
    }

    fn install(
        &self,
        kind: ArrowActionKind,
        label: Key,
        variant: ArrowActionVariant,
    ) -> ArrowAction {
        ArrowAction::new(kind, label, variant)
            .steps(&self.pick(|l| &l.install))
            .with_variables(&self.config_vars)
    }

    fn update(&self) -> ArrowAction {
        ArrowAction::new(
            ArrowActionKind::Update,
            arrow::ACTION_UPDATE,
            ArrowActionVariant::Default,
        )
        .steps(&self.pick(|l| &l.update))
    }

    fn uninstall(&self) -> ArrowAction {
        ArrowAction::new(
            ArrowActionKind::Uninstall,
            arrow::ACTION_UNINSTALL,
            ArrowActionVariant::Outline,
        )
        .steps(&self.pick(|l| &l.uninstall))
    }

    fn stop(&self) -> ArrowAction {
        ArrowAction::new(
            ArrowActionKind::Stop,
            arrow::ACTION_STOP,
            ArrowActionVariant::DestructiveOutline,
        )
        .steps(&self.pick(|l| &l.stop))
    }

    fn restart(&self) -> ArrowAction {
        let mut steps = self.pick(|l| &l.stop);
        steps.extend(self.pick(|l| &l.execute));
        ArrowAction::new(
            ArrowActionKind::Restart,
            arrow::ACTION_RESTART,
            ArrowActionVariant::Outline,
        )
        .steps(&steps)
        .with_variables(&self.config_vars)
    }

    fn remove_from_library() -> ArrowAction {
        ArrowAction::new(
            ArrowActionKind::RemoveFromLibrary,
            arrow::ACTION_REMOVE_FROM_LIBRARY,
            ArrowActionVariant::Outline,
        )
    }
}

fn add_to_library(variant: ArrowActionVariant) -> ArrowAction {
    ArrowAction::new(
        ArrowActionKind::AddToLibrary,
        arrow::ACTION_ADD_TO_LIBRARY,
        variant,
    )
}

/// The action set for the arrow's current state.
pub fn compute_actions(facts: &ArrowFacts) -> Vec<ArrowAction> {
    use ArrowActionKind as K;
    use ArrowActionVariant as V;
    let cx = Ctx {
        lifecycle: facts.lifecycle.as_ref(),
        config_vars: all_variable_names(facts),
    };

    if !facts.user_installed {
        // What the primary button is depends on the methods the arrow declares, not a fixed flow. Both
        // of these add it to the library first.
        if cx.has_install() {
            return vec![
                cx.install(K::Install, arrow::ACTION_INSTALL, V::Default),
                add_to_library(V::Outline),
            ];
        }
        if cx.has_execute() {
            return vec![cx.execute(false, V::Default), add_to_library(V::Outline)];
        }
        return vec![add_to_library(V::Default)];
    }

    let has_execute = cx.has_execute();
    match facts.state {
        RunState::Absent => {
            let primary = if !cx.has_install() && has_execute {
                cx.execute(false, V::Default)
            } else {
                cx.install(K::Install, arrow::ACTION_INSTALL, V::Default)
            };
            vec![primary, Ctx::remove_from_library()]
        }
        RunState::Installing => vec![
            cx.install(K::Install, arrow::ACTION_INSTALL, V::Default)
                .busy(arrow::ACTION_INSTALLING),
            Ctx::remove_from_library().disabled(true),
        ],
        RunState::Ready => {
            let mut actions = Vec::new();
            // `available` is the row's own record of what is ahead; the runtime's `outdated` state is
            // reconciled from it, but a detail read can land before that reconciliation does.
            if facts.update_ahead() {
                actions.push(cx.update());
            }
            if has_execute {
                actions.push(cx.execute(
                    false,
                    if facts.update_ahead() {
                        V::Outline
                    } else {
                        V::Default
                    },
                ));
            }
            actions.push(cx.uninstall());
            actions
        }
        RunState::Outdated => {
            let mut actions = vec![cx.update()];
            // Never conditional: execute is ready-only for every arrow, not gated by manifest data.
            if has_execute {
                actions.push(cx.execute(true, V::Outline));
            }
            actions
        }
        RunState::Updating => {
            let mut actions = vec![cx.update().busy(arrow::ACTION_UPDATING)];
            if has_execute {
                actions.push(cx.execute(true, V::Outline));
            }
            actions
        }
        RunState::Running => {
            let mut actions = vec![cx.stop()];
            if has_execute {
                actions.push(cx.restart());
            }
            actions
        }
        RunState::Stopping | RunState::Draining => {
            // No escalation: `BeginStop.Validate` rejects a second stop while one is in flight (409).
            let busy = if facts.state == RunState::Stopping {
                arrow::ACTION_STOPPING
            } else {
                arrow::ACTION_DRAINING
            };
            let mut actions = vec![cx.stop().busy(busy)];
            if has_execute {
                actions.push(cx.restart().disabled(true));
            }
            actions
        }
        // `BeginStop.Validate` allows `Detached` as a starting state for the ordinary stop; core has no
        // separate "force stop", so this is a plain Stop.
        RunState::Detached => vec![cx.stop()],
        RunState::Uninstalling => {
            let mut actions = Vec::new();
            if has_execute {
                actions.push(cx.execute(true, V::Default));
            }
            actions.push(cx.uninstall().busy(arrow::ACTION_UNINSTALLING));
            actions
        }
        RunState::Removed => {
            vec![cx.install(K::Reinstall, arrow::ACTION_REINSTALL, V::Outline)]
        }
    }
}
