use std::collections::BTreeMap;

use super::*;
use crate::domain::{
    ArrowLifecycle, ArrowMedia, ArrowPort, ArrowRequirement, ArrowState, ArrowTarget,
    AvailableVersion, LastReturnDetail, PortProtocol, SelectorKind, StepBase, StepProgress,
};

fn run_step(title: &str, command: Overridable<String>) -> ArrowStepDefinition {
    ArrowStepDefinition::Run {
        base: StepBase {
            title: title.into(),
            exit_on_failure: None,
        },
        command,
        elevated: Overridable::Plain(false),
        timeout: Overridable::Plain(String::new()),
    }
}

fn target(platform: &str, cores: f64) -> ArrowTarget {
    ArrowTarget {
        platform: platform.into(),
        requirement: ArrowRequirement {
            cpu_cores: cores,
            memory_gb: 4.0,
            disk_gb: 10.0,
        },
        lifecycle: ArrowLifecycle {
            execute: vec![run_step("Run", Overridable::Plain("server".into()))],
            ..ArrowLifecycle::default()
        },
        methods: vec![],
    }
}

fn detail() -> ArrowDetail {
    ArrowDetail {
        origin: None,
        confidence: None,
        warnings: vec![],
        namespace: "github.com/rabbyte/minecraft@stable".into(),
        name: "Minecraft".into(),
        description: "d".into(),
        license: "MIT".into(),
        url: "github.com/rabbyte/minecraft".into(),
        tags: vec!["game".into()],
        media: ArrowMedia {
            icon: None,
            banner: Some("b".into()),
        },
        maintainers: vec![ArrowCredit {
            name: "Rabbyte".into(),
            email: None,
            url: None,
        }],
        credits: vec![],
        netbridge: vec![ArrowPort {
            name: "game".into(),
            protocol: PortProtocol::Tcp,
            default: 7777,
            required: true,
        }],
        variables: vec![],
        targets: vec![target("linux/amd64", 2.0), target("windows/amd64", 8.0)],
        state: ArrowState::Ready,
        user_installed: true,
        installed_at: None,
        selector: "stable".into(),
        selector_kind: SelectorKind::Channel,
        resolved_ref: "v1.21.4".into(),
        installed_commit: String::new(),
        available: Some(AvailableVersion {
            reference: "v1.21.5".into(),
            commit: String::new(),
        }),
        outdated: true,
        active_run: None,
        last_return: None,
        channels: vec![],
        readme: None,
        dependencies: vec![],
        dependents: vec!["a@1".into()],
    }
}

#[test]
fn facts_follow_the_target_of_this_machine() {
    let facts = facts_from_detail(&detail(), "windows/amd64");
    assert!(facts.platform_supported);
    assert_eq!(facts.state, RunState::Ready);
    assert_eq!(facts.available_ref.as_deref(), Some("v1.21.5"));
    assert_eq!(facts.platforms, ["linux/amd64", "windows/amd64"]);
    let lifecycle = facts.lifecycle.unwrap();
    assert_eq!(lifecycle.execute[0].command.as_deref(), Some("server"));
    assert!(lifecycle.install.is_empty());
}

#[test]
fn an_unmatched_machine_falls_back_to_the_first_target_but_is_not_supported() {
    let facts = facts_from_detail(&detail(), "darwin/arm64");
    assert!(!facts.platform_supported);
    assert!(facts.lifecycle.is_some());
    assert_eq!(
        details_facts(&detail(), "darwin/arm64")
            .requirement
            .unwrap()
            .cpu_cores,
        2.0
    );
}

#[test]
fn no_targets_means_no_lifecycle_and_no_requirements() {
    let mut d = detail();
    d.targets.clear();
    let facts = facts_from_detail(&d, "linux/amd64");
    assert!(facts.lifecycle.is_none() && !facts.platform_supported);
    assert!(details_facts(&d, "linux/amd64").requirement.is_none());
}

#[test]
fn an_empty_available_ref_is_not_an_update() {
    let mut d = detail();
    d.available = Some(AvailableVersion {
        reference: String::new(),
        commit: String::new(),
    });
    assert!(!facts_from_detail(&d, "linux/amd64").update_ahead());
}

#[test]
fn per_platform_commands_resolve_to_the_machine_then_default() {
    let command = Overridable::PerPlatform(BTreeMap::from([
        ("default".to_owned(), "sh run".to_owned()),
        ("windows/amd64".to_owned(), "run.exe".to_owned()),
    ]));
    let step = run_step("Run", command);
    assert_eq!(
        step_spec(&step, "windows/amd64").command.unwrap(),
        "run.exe"
    );
    assert_eq!(step_spec(&step, "linux/amd64").command.unwrap(), "sh run");
}

#[test]
fn the_step_inspector_gets_the_manifest_text() {
    let spec = step_spec(&run_step("Run", Overridable::Plain("server".into())), "x");
    assert_eq!(spec.step_type, "run");
    assert_eq!(spec.code, "run:\n  title: Run\n  command: server");
}

#[test]
fn the_live_run_becomes_run_facts() {
    let mut d = detail();
    d.active_run = Some(ActiveRun {
        method: "update".into(),
        pid: None,
        variables: BTreeMap::new(),
        steps: vec![StepProgress {
            index: 0,
            title: "Download".into(),
            status: WireStepStatus::Running,
            step_type: "fetch".into(),
            error: None,
            note: None,
        }],
        surface: None,
    });
    let run = facts_from_detail(&d, "linux/amd64").active_run.unwrap();
    assert_eq!(run.steps[0].status, StepStatus::Running);
}

#[test]
fn a_failed_last_run_reports_the_failed_steps_error() {
    let mut d = detail();
    let step = |status, error: Option<&str>| StepProgress {
        index: 0,
        title: "s".into(),
        status,
        step_type: "run".into(),
        error: error.map(Into::into),
        note: None,
    };
    d.last_return = Some(LastReturnDetail {
        method: "execute".into(),
        outcome: Outcome::Failed,
        variables: BTreeMap::new(),
        steps: vec![
            step(WireStepStatus::Completed, None),
            step(WireStepStatus::Failed, Some("exit 1")),
        ],
    });
    let last = last_run_facts(&d).unwrap();
    assert!(last.failed);
    assert_eq!(last.failed_step_error.as_deref(), Some("exit 1"));
    d.last_return = None;
    assert!(last_run_facts(&d).is_none());
}

#[test]
fn details_carry_ports_people_and_graph_counts() {
    let facts = details_facts(&detail(), "linux/amd64");
    assert_eq!(facts.ports[0].default, 7777);
    assert_eq!(facts.ports[0].protocol, "tcp");
    assert_eq!(facts.maintainers[0].name, "Rabbyte");
    assert_eq!((facts.depends_on, facts.required_by), (0, 1));
}
