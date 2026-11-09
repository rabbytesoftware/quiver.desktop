use super::facts::fixtures::{discovered, installed};
use super::facts::*;
use super::header::*;
use super::status::{ArrowProblem, LastRunFacts, RunState, StatusGlyph};
use crate::i18n::{en::arrow, en::design, Translator};

fn cx() -> HeaderContext<'static> {
    HeaderContext {
        platform: "darwin/arm64",
        platform_resolved: true,
        ..HeaderContext::default()
    }
}

#[test]
fn identity_status_tags_and_description_come_straight_from_the_arrow() {
    let facts = ArrowFacts {
        tags: vec!["game".into(), "server".into()],
        description: "A server.".into(),
        resolved_ref: "v1.21.4".into(),
        banner: Some("https://x/banner.png".into()),
        license: "MIT".into(),
        ..installed(RunState::Running)
    };
    let header = build_header(&facts, &cx());
    assert_eq!(header.name, "Caddy");
    assert_eq!(header.status.label, arrow::STATE_RUNNING);
    assert_eq!(header.status.glyph, StatusGlyph::Running);
    assert_eq!(header.tags, ["game", "server"]);
    assert_eq!(header.banner.as_deref(), Some("https://x/banner.png"));
    assert_eq!(header.license.as_deref(), Some("MIT"));
    assert!(header.problem.is_none());
}

#[test]
fn an_empty_banner_or_license_is_absent() {
    let facts = ArrowFacts {
        banner: Some(String::new()),
        ..installed(RunState::Ready)
    };
    let header = build_header(&facts, &cx());
    assert!(header.banner.is_none() && header.license.is_none());
}

#[test]
fn the_platform_badge_waits_for_the_resolved_platform() {
    let facts = ArrowFacts {
        platform_supported: false,
        ..installed(RunState::Ready)
    };
    let guess = HeaderContext {
        platform_resolved: false,
        ..cx()
    };
    assert!(build_header(&facts, &guess).platform_badge.is_none());
    let badge = build_header(&facts, &cx()).platform_badge.unwrap();
    assert_eq!(
        badge.text(&Translator::default()),
        "Not available for this Mac"
    );
    assert!(build_header(&installed(RunState::Ready), &cx())
        .platform_badge
        .is_none());
}

#[test]
fn before_the_library_the_version_is_a_picker_while_channels_exist_or_load() {
    let loading = HeaderContext {
        channels_loading: true,
        ..cx()
    };
    assert_eq!(
        build_header(&discovered(), &loading).version,
        VersionControl::Picker { loading: true }
    );
    let loaded = HeaderContext {
        has_channels: true,
        ..cx()
    };
    assert_eq!(
        build_header(&discovered(), &loaded).version,
        VersionControl::Picker { loading: false }
    );
    assert_eq!(
        build_header(&discovered(), &cx()).version,
        VersionControl::None
    );
}

#[test]
fn in_the_library_the_version_is_read_only_with_a_switch_when_published() {
    let facts = ArrowFacts {
        resolved_ref: "v1".into(),
        ..installed(RunState::Ready)
    };
    let with_channels = HeaderContext {
        has_channels: true,
        selector: "stable",
        ..cx()
    };
    let VersionControl::Fixed {
        version,
        selector,
        switch,
    } = build_header(&facts, &with_channels).version
    else {
        panic!("expected a fixed version");
    };
    assert_eq!(
        (version.as_str(), selector.as_str(), switch),
        ("v1", "stable", SwitchAffordance::Enabled)
    );
    let running = ArrowFacts {
        state: RunState::Running,
        ..facts.clone()
    };
    let VersionControl::Fixed { switch, .. } = build_header(&running, &with_channels).version
    else {
        panic!()
    };
    assert_eq!(switch, SwitchAffordance::Disabled);
    let VersionControl::Fixed { switch, .. } = build_header(&facts, &cx()).version else {
        panic!()
    };
    assert_eq!(switch, SwitchAffordance::Hidden);
    let switching = HeaderContext {
        switching: true,
        ..with_channels
    };
    let VersionControl::Fixed { switch, .. } = build_header(&facts, &switching).version else {
        panic!()
    };
    assert_eq!(switch, SwitchAffordance::Disabled);
}

#[test]
fn a_detached_arrow_or_a_failed_run_adds_the_problem_chip() {
    let header = build_header(&installed(RunState::Detached), &cx());
    assert_eq!(header.problem, Some(ArrowProblem::Detached));
    assert_eq!(header.status.label, design::STATUS_NEEDS_ATTENTION);
    let failed = HeaderContext {
        last_run: Some(LastRunFacts {
            failed: true,
            failed_step_error: Some("exit 2".into()),
        }),
        ..cx()
    };
    let header = build_header(&installed(RunState::Ready), &failed);
    let tr = Translator::default();
    assert_eq!(problem_text(&header.problem.unwrap(), &tr), "exit 2");
    assert_eq!(
        problem_text(&ArrowProblem::Failed { detail: None }, &tr),
        tr.t(arrow::PROBLEM_FAILED_NOTE)
    );
    assert_eq!(
        problem_text(&ArrowProblem::Detached, &tr),
        tr.t(arrow::PROBLEM_DETACHED_NOTE)
    );
}
