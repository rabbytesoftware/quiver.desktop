use super::*;

const BUILT: i64 = 1_791_122_529; // 2026-10-04T14:02:09Z

fn build(channel: BuildChannel, version: Option<&str>) -> BuildDescriptor {
    BuildDescriptor {
        channel,
        version: version.map(Into::into),
        commit: Some("abcdef0123456789".into()),
        built_at: Some(BUILT),
    }
}

fn rest(b: &BuildDescriptor) -> String {
    format_build(b, FormatOptions::default())
}

#[test]
fn channels_normalise_the_way_the_pipeline_names_them() {
    for (raw, want) in [
        ("stable", BuildChannel::Stable),
        (" Beta ", BuildChannel::Beta),
        ("hotfix", BuildChannel::Hotfix),
        ("nightly-rolling", BuildChannel::Nightly),
        ("nightly-latest", BuildChannel::Nightly),
    ] {
        assert_eq!(normalise_channel(Some(raw)), Some(want), "{raw}");
    }
    assert_eq!(normalise_channel(Some("")), None);
    assert_eq!(normalise_channel(Some("preview")), None);
    assert_eq!(normalise_channel(None), None);
}

#[test]
fn the_channel_comes_off_a_release_tag_and_nothing_else() {
    assert_eq!(
        channel_of_label(Some("beta-26.5-4")),
        Some(BuildChannel::Beta)
    );
    assert_eq!(
        channel_of_label(Some("stable-26.5.1")),
        Some(BuildChannel::Stable)
    );
    assert_eq!(channel_of_label(Some("26.5.1")), None);
    assert_eq!(channel_of_label(Some("nightly-latest")), None);
    assert_eq!(channel_of_label(None), None);
}

#[test]
fn versions_parse_with_or_without_a_prefix_and_a_build_count() {
    let p = |v| parse_version(Some(v));
    assert_eq!(
        p("26.5.1"),
        Some(ParsedVersion {
            series: "26.5.1".into(),
            build: None
        })
    );
    assert_eq!(
        p("beta-26.5-4"),
        Some(ParsedVersion {
            series: "26.5".into(),
            build: Some(4)
        })
    );
    assert_eq!(
        p("2026-09-27"),
        Some(ParsedVersion {
            series: "2026-09-27".into(),
            build: None
        })
    );
    assert_eq!(
        p("hotfix-2026-09-27-2"),
        Some(ParsedVersion {
            series: "2026-09-27".into(),
            build: Some(2)
        })
    );
    for bad in ["", "latest", "26", "1.2.x", "v1.2"] {
        assert_eq!(p(bad), None, "{bad}");
    }
}

#[test]
fn the_build_time_is_month_day_and_time_at_the_viewers_offset() {
    assert_eq!(format_build_time(BUILT, false, 0), "10-04 14:02");
    assert_eq!(format_build_time(BUILT, true, 0), "10-04");
    assert_eq!(format_build_time(BUILT, false, 120), "10-04 16:02");
    assert_eq!(format_build_time(1_767_324_960, false, 0), "01-02 03:36");
    assert_eq!(
        format_build_time(1_791_153_600, true, -600),
        "10-04",
        "a negative offset"
    );
}

#[test]
fn a_rolling_build_shows_its_timestamp_and_only_the_channel_without_one() {
    assert_eq!(
        rest(&build(BuildChannel::Nightly, None)),
        "nightly 10-04 14:02"
    );
    let bare = BuildDescriptor {
        built_at: None,
        ..build(BuildChannel::Nightly, None)
    };
    assert_eq!(rest(&bare), "nightly");
}

#[test]
fn a_release_shows_its_version_never_its_timestamp() {
    assert_eq!(
        rest(&build(BuildChannel::Stable, Some("stable-26.5.1"))),
        "stable 26.5.1"
    );
    assert_eq!(
        rest(&build(BuildChannel::Beta, Some("beta-26.5-4"))),
        "beta 26.5 #4"
    );
    assert_eq!(
        rest(&build(BuildChannel::Hotfix, Some("26.5-2"))),
        "hotfix 26.5 #2"
    );
    assert_eq!(rest(&build(BuildChannel::Stable, None)), "stable");
    assert_eq!(
        rest(&build(BuildChannel::Beta, Some("weird"))),
        "beta weird",
        "an unparseable version is kept"
    );
}

#[test]
fn an_unstamped_build_says_dev_whatever_else_it_knows() {
    assert_eq!(rest(&build(BuildChannel::Dev, Some("26.5.1"))), "dev");
}

#[test]
fn a_narrow_rail_drops_the_time_of_day_and_leaves_releases_alone() {
    let compact = FormatOptions {
        compact: true,
        ..FormatOptions::default()
    };
    assert_eq!(
        format_build(&build(BuildChannel::Nightly, None), compact),
        "nightly 10-04"
    );
    assert_eq!(
        format_build(&build(BuildChannel::Beta, Some("26.5-4")), compact),
        "beta 26.5 #4"
    );
    assert!(compact_for_width(159.0) && !compact_for_width(160.0));
}

#[test]
fn hover_swaps_in_the_short_commit_for_every_channel_and_falls_back_to_rest() {
    let hover = FormatOptions {
        hover: true,
        compact: true,
        ..FormatOptions::default()
    };
    assert_eq!(
        format_build(&build(BuildChannel::Nightly, None), hover),
        "nightly abcdef0",
        "not narrowed by compact"
    );
    assert_eq!(
        format_build(&build(BuildChannel::Beta, Some("26.5-4")), hover),
        "beta abcdef0"
    );
    let short = BuildDescriptor {
        commit: Some("abc".into()),
        ..build(BuildChannel::Dev, None)
    };
    assert_eq!(format_build(&short, hover), "dev abc");
    let none = BuildDescriptor {
        commit: None,
        ..build(BuildChannel::Stable, Some("26.5.1"))
    };
    assert_eq!(format_build(&none, hover), "stable 26.5.1");
}

#[test]
fn the_desktop_takes_its_channel_from_the_build_channel_then_the_label_then_is_dev() {
    let stamp = |channel: &str, label: Option<&str>| DesktopStamp {
        channel: channel.into(),
        label: label.map(Into::into),
        ..DesktopStamp::default()
    };
    assert_eq!(
        describe_desktop(&stamp("beta", Some("beta-26.5-4"))).channel,
        BuildChannel::Beta
    );
    assert_eq!(
        describe_desktop(&stamp("", Some("hotfix-26.5-1"))).channel,
        BuildChannel::Hotfix
    );
    assert_eq!(
        describe_desktop(&stamp("", None)).channel,
        BuildChannel::Dev
    );
    assert_eq!(
        describe_desktop(&stamp("stable", Some("beta-26.5-4"))).channel,
        BuildChannel::Stable,
        "the pipeline channel wins over a label that disagrees"
    );
}

#[test]
fn the_core_is_unknown_until_it_answers_and_reads_versions() {
    assert_eq!(describe_core(None), None);
    let info = CoreVersions {
        version: "nightly-latest".into(),
        commit: "abcdef0123".into(),
        built_at: "2026-10-04T14:02:09Z".into(),
        channel: "nightly-latest".into(),
        ..CoreVersions::default()
    };
    let d = describe_core(Some(&info)).unwrap();
    assert_eq!(
        (d.channel, d.built_at, d.commit.as_deref()),
        (BuildChannel::Nightly, Some(BUILT), Some("abcdef0123"))
    );
    let bad_time = CoreVersions {
        built_at: "soon".into(),
        ..info.clone()
    };
    assert_eq!(describe_core(Some(&bad_time)).unwrap().built_at, None);
}

#[test]
fn a_core_without_a_channel_derives_it_from_a_release_version_else_is_dev() {
    let core = |version: &str| {
        describe_core(Some(&CoreVersions {
            version: version.into(),
            ..CoreVersions::default()
        }))
        .unwrap()
    };
    assert_eq!(core("beta-26.5-4").channel, BuildChannel::Beta);
    assert_eq!(core("0.1.0").channel, BuildChannel::Dev);
    assert_eq!(core("").version, None);
}

#[test]
fn the_long_form_lists_everything_known() {
    let long = describe_build_long(&build(BuildChannel::Beta, Some("beta-26.5-4")));
    assert_eq!(
        long,
        "beta 26.5 #4 · abcdef0123456789 · 2026-10-04T14:02:09Z"
    );
    let bare = BuildDescriptor {
        commit: None,
        built_at: None,
        ..build(BuildChannel::Dev, None)
    };
    assert_eq!(describe_build_long(&bare), "dev");
}

#[test]
fn the_indicator_is_quiet_on_a_stable_release_only() {
    assert!(!indicator_shown_by_default(BuildChannel::Stable));
    for c in [
        BuildChannel::Beta,
        BuildChannel::Hotfix,
        BuildChannel::Nightly,
        BuildChannel::Dev,
    ] {
        assert!(indicator_shown_by_default(c));
    }
}

#[test]
fn choosing_the_default_stores_nothing_so_a_channel_change_follows_the_new_default() {
    let mut on_stable = BuildIndicatorSetting::new(None, BuildChannel::Stable);
    assert!(!on_stable.shown() && !on_stable.changed());
    on_stable.set(true);
    assert_eq!(
        (on_stable.shown(), on_stable.stored(), on_stable.changed()),
        (true, Some(true), true)
    );
    on_stable.set(false);
    assert_eq!(on_stable.stored(), None);
    let mut nightly = BuildIndicatorSetting::new(Some(false), BuildChannel::Nightly);
    assert!(!nightly.shown() && nightly.changed());
    nightly.reset();
    assert!(nightly.shown());
}
