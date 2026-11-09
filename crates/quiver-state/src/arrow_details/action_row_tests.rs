use super::action_row::*;
use super::actions::ArrowActionKind as K;
use super::facts::fixtures::{discovered, installed};
use super::facts::*;
use super::status::RunState;
use crate::i18n::{en::arrow, en::design};

fn updating_with_run() -> ArrowFacts {
    let step = |title: &str, status| RunStepFacts {
        title: title.into(),
        step_type: "fetch".into(),
        status,
        error: None,
        note: None,
    };
    ArrowFacts {
        active_run: Some(ActiveRunFacts {
            method: "update".into(),
            steps: vec![
                step("Download", StepStatus::Completed),
                step("Verify checksum", StepStatus::Running),
                step("Extract", StepStatus::Pending),
                step("Restart", StepStatus::Pending),
            ],
        }),
        ..installed(RunState::Updating)
    }
}

#[test]
fn ready_row_has_start_and_uninstall_each_with_info() {
    let row = build_action_row(&installed(RunState::Ready), None, false);
    let start = row.button(K::Execute).unwrap();
    assert!(start.enabled && !start.busy);
    assert_eq!(start.info.as_ref().unwrap().title, arrow::ACTION_START);
    assert!(row.progress().is_none());
    assert!(row.show_methods);
    assert!(!row.show_settings);
}

#[test]
fn settings_icon_appears_only_when_variables_are_declared() {
    let facts = ArrowFacts {
        variable_names: vec!["PORT".into()],
        ..installed(RunState::Ready)
    };
    assert!(build_action_row(&facts, None, false).show_settings);
}

#[test]
fn add_to_library_has_no_info() {
    let row = build_action_row(&discovered(), None, false);
    assert!(row.button(K::AddToLibrary).unwrap().info.is_none());
    assert!(row.button(K::Install).unwrap().info.is_some());
}

#[test]
fn the_running_update_becomes_the_progress_control() {
    let row = build_action_row(&updating_with_run(), None, false);
    let progress = row.progress().unwrap();
    assert_eq!(progress.verb, design::VERB_UPDATING);
    assert_eq!(progress.index, 2);
    assert!(row.button(K::Update).is_none());
    let start = row.button(K::Execute).unwrap();
    assert!(!start.enabled);
}

#[test]
fn a_request_in_flight_shows_an_indeterminate_progress_control() {
    let row = build_action_row(&installed(RunState::Absent), Some(K::Install), false);
    let progress = row.progress().unwrap();
    assert_eq!(progress.verb, design::VERB_INSTALLING);
    assert_eq!(progress.fraction, None);
}

#[test]
fn start_in_flight_stays_a_busy_disabled_button() {
    let row = build_action_row(&installed(RunState::Ready), Some(K::Execute), false);
    assert!(row.progress().is_none());
    let start = row.button(K::Execute).unwrap();
    assert!(start.busy && !start.enabled);
}

#[test]
fn stopping_and_draining_pick_their_own_verb() {
    let row = build_action_row(&installed(RunState::Draining), None, false);
    assert_eq!(row.progress().unwrap().verb, design::VERB_DRAINING);
    let row = build_action_row(&installed(RunState::Stopping), None, false);
    assert_eq!(row.progress().unwrap().verb, design::VERB_STOPPING);
}

#[test]
fn a_selector_switch_disables_every_button() {
    let row = build_action_row(&installed(RunState::Ready), None, true);
    assert!(row.slots.iter().all(|slot| match slot {
        ActionSlot::Button(b) => !b.enabled,
        ActionSlot::Progress(_) => true,
    }));
}

#[test]
fn needs_attention_offers_one_real_stop() {
    let row = build_action_row(&installed(RunState::Detached), None, false);
    assert_eq!(row.slots.len(), 1);
    assert!(row.button(K::Stop).unwrap().enabled);
}
