use super::actions::*;
use super::facts::fixtures::{discovered, installed, step};
use super::facts::*;
use super::status::RunState;
use crate::i18n::en::arrow;

fn kinds(facts: &ArrowFacts) -> Vec<ArrowActionKind> {
    compute_actions(facts).iter().map(|a| a.kind).collect()
}

fn find(facts: &ArrowFacts, kind: ArrowActionKind) -> ArrowAction {
    compute_actions(facts)
        .into_iter()
        .find(|a| a.kind == kind)
        .unwrap()
}

use ArrowActionKind as K;

#[test]
fn not_in_the_library_install_is_primary_and_add_is_secondary() {
    let actions = compute_actions(&discovered());
    assert_eq!(actions[0].kind, K::Install);
    assert_eq!(actions[0].variant, ArrowActionVariant::Default);
    assert_eq!(actions[1].kind, K::AddToLibrary);
    assert_eq!(actions[1].variant, ArrowActionVariant::Outline);
    assert!(actions[1].steps.is_empty());
}

#[test]
fn not_in_the_library_without_install_start_is_primary() {
    let mut facts = discovered();
    facts.lifecycle.as_mut().unwrap().install.clear();
    assert_eq!(kinds(&facts), [K::Execute, K::AddToLibrary]);
}

#[test]
fn not_in_the_library_with_nothing_to_run_only_adds() {
    let mut facts = discovered();
    facts.lifecycle = None;
    let actions = compute_actions(&facts);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].variant, ArrowActionVariant::Default);
}

#[test]
fn an_execute_that_launches_the_desktop_app_is_open() {
    let mut facts = installed(RunState::Ready);
    facts.lifecycle.as_mut().unwrap().execute = vec![StepSpec {
        command: Some("${ARROW_APP}".into()),
        ..step("Launch", "run")
    }];
    assert_eq!(find(&facts, K::Execute).label, arrow::ACTION_OPEN);
    assert_eq!(
        find(&installed(RunState::Ready), K::Execute).label,
        arrow::ACTION_START
    );
}

#[test]
fn absent_offers_install_and_remove() {
    assert_eq!(
        kinds(&installed(RunState::Absent)),
        [K::Install, K::RemoveFromLibrary]
    );
}

#[test]
fn installing_is_busy_and_remove_is_disabled() {
    let facts = installed(RunState::Installing);
    let install = find(&facts, K::Install);
    assert!(install.force_busy);
    assert_eq!(install.busy_label, Some(arrow::ACTION_INSTALLING));
    assert!(find(&facts, K::RemoveFromLibrary).force_disabled);
}

#[test]
fn ready_offers_start_and_uninstall_and_omits_start_without_an_execute() {
    let facts = installed(RunState::Ready);
    assert_eq!(kinds(&facts), [K::Execute, K::Uninstall]);
    assert!(!find(&facts, K::Execute).force_disabled);
    let mut no_exec = facts;
    no_exec.lifecycle.as_mut().unwrap().execute.clear();
    assert_eq!(kinds(&no_exec), [K::Uninstall]);
}

#[test]
fn ready_with_something_ahead_puts_update_first() {
    let mut facts = installed(RunState::Ready);
    facts.available_ref = Some("v2".into());
    assert_eq!(kinds(&facts), [K::Update, K::Execute, K::Uninstall]);
    assert_eq!(
        find(&facts, K::Execute).variant,
        ArrowActionVariant::Outline
    );
    facts.available_ref = None;
    assert!(!kinds(&facts).contains(&K::Update));
}

#[test]
fn outdated_and_updating_hard_disable_start() {
    let outdated = installed(RunState::Outdated);
    assert_eq!(kinds(&outdated), [K::Update, K::Execute]);
    assert!(find(&outdated, K::Execute).force_disabled);
    let updating = installed(RunState::Updating);
    let update = find(&updating, K::Update);
    assert!(update.force_busy);
    assert_eq!(update.busy_label, Some(arrow::ACTION_UPDATING));
    assert!(find(&updating, K::Execute).force_disabled);
}

#[test]
fn running_offers_stop_and_restart_with_both_step_lists() {
    let facts = installed(RunState::Running);
    assert_eq!(kinds(&facts), [K::Stop, K::Restart]);
    let restart = find(&facts, K::Restart);
    assert_eq!(restart.steps.len(), 2);
    assert_eq!(restart.steps[0].title, "Signal");
    let mut no_exec = facts;
    no_exec.lifecycle.as_mut().unwrap().execute.clear();
    assert_eq!(kinds(&no_exec), [K::Stop]);
}

#[test]
fn stopping_and_draining_are_busy_with_a_disabled_restart_and_no_escalation() {
    for (state, label) in [
        (RunState::Stopping, arrow::ACTION_STOPPING),
        (RunState::Draining, arrow::ACTION_DRAINING),
    ] {
        let facts = installed(state);
        assert_eq!(kinds(&facts), [K::Stop, K::Restart]);
        assert_eq!(find(&facts, K::Stop).busy_label, Some(label));
        assert!(find(&facts, K::Restart).force_disabled);
    }
}

#[test]
fn detached_is_a_single_plain_stop() {
    let facts = installed(RunState::Detached);
    let actions = compute_actions(&facts);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].kind, K::Stop);
    assert!(!actions[0].force_disabled && !actions[0].force_busy);
}

#[test]
fn uninstalling_disables_start_and_marks_uninstall_busy() {
    let facts = installed(RunState::Uninstalling);
    assert_eq!(kinds(&facts), [K::Execute, K::Uninstall]);
    assert!(find(&facts, K::Execute).force_disabled);
    assert_eq!(
        find(&facts, K::Uninstall).busy_label,
        Some(arrow::ACTION_UNINSTALLING)
    );
}

#[test]
fn removed_offers_only_reinstall_with_the_install_steps() {
    let facts = installed(RunState::Removed);
    let actions = compute_actions(&facts);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].kind, K::Reinstall);
    assert_eq!(actions[0].steps.len(), 2);
}

#[test]
fn variables_are_all_or_nothing() {
    let mut facts = installed(RunState::Absent);
    facts.variable_names = vec!["A".into(), "B".into()];
    assert_eq!(find(&facts, K::Install).uses_variables, ["A", "B"]);
    assert!(find(&facts, K::RemoveFromLibrary).uses_variables.is_empty());
    let ready = ArrowFacts {
        state: RunState::Ready,
        ..facts
    };
    assert!(find(&ready, K::Uninstall).uses_variables.is_empty());
}

#[test]
fn no_targets_means_empty_step_lists_not_a_panic() {
    for state in [
        RunState::Absent,
        RunState::Detached,
        RunState::Running,
        RunState::Removed,
        RunState::Uninstalling,
    ] {
        let mut facts = installed(state);
        facts.lifecycle = None;
        for action in compute_actions(&facts) {
            assert!(action.steps.is_empty());
        }
    }
    let mut running = installed(RunState::Running);
    running.lifecycle = None;
    assert_eq!(kinds(&running), [K::Stop]);
}

#[test]
fn quivers_own_row_leaves_the_release_variables_out_of_the_form() {
    let mut facts = installed(RunState::Absent);
    facts.namespace = "github.com/rabbytesoftware/quiver.desktop@stable".into();
    facts.variable_names = vec![
        "QUIVER_RELEASE_ASSET_URL".into(),
        "QUIVER_RELEASE_CHECKSUM".into(),
        "OTHER".into(),
    ];
    assert_eq!(find(&facts, K::Install).uses_variables, ["OTHER"]);
    facts.state = RunState::Removed;
    assert_eq!(find(&facts, K::Reinstall).uses_variables, ["OTHER"]);
    let other = ArrowFacts {
        namespace: "github.com/rabbyte/caddy".into(),
        state: RunState::Absent,
        ..facts
    };
    assert_eq!(find(&other, K::Install).uses_variables.len(), 3);
}

#[test]
fn quiver_cores_row_offers_the_same_actions_and_a_runnable_update() {
    for state in [
        RunState::Absent,
        RunState::Installing,
        RunState::Updating,
        RunState::Ready,
        RunState::Running,
        RunState::Stopping,
        RunState::Draining,
        RunState::Detached,
        RunState::Uninstalling,
        RunState::Removed,
        RunState::Outdated,
    ] {
        let other = ArrowFacts {
            available_ref: Some("stable-26.6".into()),
            ..installed(state)
        };
        let core = ArrowFacts {
            namespace: "github.com/rabbytesoftware/quiver.core@stable".into(),
            ..other.clone()
        };
        assert_eq!(compute_actions(&core), compute_actions(&other), "{state:?}");
    }
    let core = ArrowFacts {
        namespace: "github.com/rabbytesoftware/quiver.core@stable".into(),
        ..installed(RunState::Outdated)
    };
    assert!(!find(&core, K::Update).force_disabled);
}
