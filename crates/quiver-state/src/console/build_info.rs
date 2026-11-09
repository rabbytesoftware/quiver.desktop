//! What the rail's build indicator says about a build, as a pure function.
//!
//! Two builds are shown, `core` (the daemon) and `app` (this desktop). At rest a rolling build (nightly)
//! reports WHEN it was built, because its tag never changes and a timestamp is the only thing that tells
//! two nightlies apart; a release reports its version and name. On hover either one reports the commit.
//! An unstamped build says `dev`.

use super::frames::parse_rfc3339_ms;
use super::versions::CoreVersions;

/// The full line needs about 160px (label + `nightly MM-DD HH:mm` at 10px mono); below that the time of
/// day is dropped.
pub const COMPACT_BELOW_PX: f64 = 160.0;
pub const SHORT_COMMIT: usize = 7;

pub fn compact_for_width(rail_px: f64) -> bool {
    rail_px < COMPACT_BELOW_PX
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuildChannel {
    Stable,
    Beta,
    Hotfix,
    Nightly,
    Dev,
}

impl BuildChannel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Hotfix => "hotfix",
            Self::Nightly => "nightly",
            Self::Dev => "dev",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildDescriptor {
    pub channel: BuildChannel,
    /// The release version: a tag (`stable-26.5.1`, `beta-26.5-4`) or the bare form of one (`26.5.1`,
    /// `26.5-4`). Only meaningful for stable, beta and hotfix.
    pub version: Option<String>,
    /// Full or short commit; the indicator shows the first [`SHORT_COMMIT`] characters.
    pub commit: Option<String>,
    /// Unix seconds.
    pub built_at: Option<i64>,
}

/// `nightly-rolling` is what the pipeline calls the rolling channel; the indicator calls it `nightly`.
pub fn normalise_channel(raw: Option<&str>) -> Option<BuildChannel> {
    match raw?.trim().to_lowercase().as_str() {
        "stable" => Some(BuildChannel::Stable),
        "beta" => Some(BuildChannel::Beta),
        "hotfix" => Some(BuildChannel::Hotfix),
        "nightly" | "nightly-rolling" | "nightly-latest" => Some(BuildChannel::Nightly),
        _ => None,
    }
}

fn strip_channel_prefix(label: &str) -> (Option<BuildChannel>, &str) {
    for (prefix, channel) in [
        ("stable-", BuildChannel::Stable),
        ("beta-", BuildChannel::Beta),
        ("hotfix-", BuildChannel::Hotfix),
    ] {
        if let Some(rest) = label.strip_prefix(prefix) {
            return (Some(channel), rest);
        }
    }
    (None, label)
}

/// The channel a release tag belongs to, or `None` when it is not one.
pub fn channel_of_label(label: Option<&str>) -> Option<BuildChannel> {
    strip_channel_prefix(label?.trim()).0
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParsedVersion {
    /// `26.5.1`, `26.5`, `2026-09-27`.
    pub series: String,
    /// The build count of a beta or hotfix tag (`beta-26.5-4` -> 4).
    pub build: Option<u32>,
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `\d+(\.\d+)+` or `\d{4}-\d{2}-\d{2}`.
fn is_series(s: &str) -> bool {
    let dotted = s.contains('.') && s.split('.').all(all_digits);
    let parts: Vec<&str> = s.split('-').collect();
    let dated = parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| all_digits(p));
    dotted || dated
}

/// Reads a tag or bare version; `None` when it does not have the shape of one.
pub fn parse_version(version: Option<&str>) -> Option<ParsedVersion> {
    let (_, bare) = strip_channel_prefix(version.unwrap_or_default().trim());
    if is_series(bare) {
        return Some(ParsedVersion {
            series: bare.to_owned(),
            build: None,
        });
    }
    let (series, count) = bare.rsplit_once('-')?;
    (is_series(series) && all_digits(count)).then(|| ParsedVersion {
        series: series.to_owned(),
        build: count.parse().ok(),
    })
}

fn short_commit(commit: Option<&str>) -> Option<String> {
    let trimmed = commit?.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(SHORT_COMMIT).collect())
}

/// `MM-DD HH:mm` at the viewer's UTC offset (minutes); without the time when `compact`.
pub fn format_build_time(seconds: i64, compact: bool, offset_minutes: i32) -> String {
    let secs = seconds + i64::from(offset_minutes) * 60;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let date = format!("{month:02}-{day:02}");
    if compact {
        date
    } else {
        format!("{date} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FormatOptions {
    /// Show the commit instead of the version or timestamp.
    pub hover: bool,
    /// Narrow rail: drop the time of day from a timestamp.
    pub compact: bool,
    /// The viewer's offset from UTC, in minutes.
    pub offset_minutes: i32,
}

/// The text for one build line, without its `core`/`app` label.
pub fn format_build(build: &BuildDescriptor, options: FormatOptions) -> String {
    let channel = build.channel.name();
    if options.hover {
        if let Some(hash) = short_commit(build.commit.as_deref()) {
            return format!("{channel} {hash}");
        }
        // Nothing to reveal: say the same thing the build says at rest.
    }
    match build.channel {
        BuildChannel::Dev => "dev".into(),
        BuildChannel::Nightly => match build.built_at {
            None => "nightly".into(),
            Some(at) => format!(
                "nightly {}",
                format_build_time(at, options.compact, options.offset_minutes)
            ),
        },
        _ => match parse_version(build.version.as_deref()) {
            None => match build
                .version
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                Some(v) => format!("{channel} {v}"),
                None => channel.to_owned(),
            },
            Some(ParsedVersion {
                series,
                build: None,
            }) => format!("{channel} {series}"),
            Some(ParsedVersion {
                series,
                build: Some(n),
            }) => format!("{channel} {series} #{n}"),
        },
    }
}

/// What the desktop build knows about itself.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DesktopStamp {
    /// The pipeline's build channel, trimmed; empty for a PR or local build.
    pub channel: String,
    /// The release label from the native build stamp.
    pub label: Option<String>,
    pub commit: Option<String>,
    pub built_at: Option<i64>,
}

/// The desktop's descriptor. A build with neither a known channel nor a release label is `dev`.
pub fn describe_desktop(stamp: &DesktopStamp) -> BuildDescriptor {
    BuildDescriptor {
        channel: normalise_channel(Some(&stamp.channel))
            .or_else(|| channel_of_label(stamp.label.as_deref()))
            .unwrap_or(BuildChannel::Dev),
        version: stamp.label.clone(),
        commit: stamp.commit.clone(),
        built_at: stamp.built_at,
    }
}

/// The core's descriptor; a daemon that reports no channel is `dev`. `None` until it has answered.
pub fn describe_core(info: Option<&CoreVersions>) -> Option<BuildDescriptor> {
    let info = info?;
    let non_empty = |s: &str| (!s.is_empty()).then(|| s.to_owned());
    Some(BuildDescriptor {
        channel: normalise_channel(Some(&info.channel))
            .or_else(|| channel_of_label(Some(&info.version)))
            .unwrap_or(BuildChannel::Dev),
        version: non_empty(&info.version),
        commit: non_empty(&info.commit),
        built_at: parse_rfc3339_ms(&info.built_at).map(|ms| ms.div_euclid(1000)),
    })
}

/// The long form, for a tooltip: everything known about the build.
pub fn describe_build_long(build: &BuildDescriptor) -> String {
    let plain = BuildDescriptor {
        built_at: None,
        commit: None,
        ..build.clone()
    };
    let mut parts = vec![format_build(&plain, FormatOptions::default())];
    if let Some(commit) = &build.commit {
        parts.push(commit.chars().take(40).collect());
    }
    if let Some(at) = build.built_at {
        parts.push(format_build_time_iso(at));
    }
    parts.join(" · ")
}

fn format_build_time_iso(seconds: i64) -> String {
    let date = format_build_time(seconds, true, 0);
    let rem = seconds.rem_euclid(86_400);
    format!(
        "{}-{date}T{:02}:{:02}:{:02}Z",
        utc_year(seconds),
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn utc_year(seconds: i64) -> i64 {
    let z = seconds.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    yoe + era * 400 + i64::from(month <= 2)
}

/// Whether the build indicator is shown to someone who has not chosen. A stable release is for people
/// using Quiver, so it stays quiet; every other channel (beta, hotfix, nightly and an unstamped dev build)
/// is for people who want to know exactly which build they are on, so it is shown.
pub fn indicator_shown_by_default(channel: BuildChannel) -> bool {
    channel != BuildChannel::Stable
}

/// The build indicator's on/off setting. Nothing is stored until the person picks something other than
/// the default, so a build that changes channel (a nightly user moving to stable) follows its new default
/// instead of carrying an old build's behaviour along.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BuildIndicatorSetting {
    /// `None` means the person has not chosen, so the build's own channel decides.
    stored: Option<bool>,
    shown_by_default: bool,
}

pub const BUILD_INDICATOR_STORAGE_KEY: &str = "quiver.buildIndicator";

impl BuildIndicatorSetting {
    pub fn new(stored: Option<bool>, build_channel: BuildChannel) -> Self {
        Self {
            stored,
            shown_by_default: indicator_shown_by_default(build_channel),
        }
    }

    pub fn shown(&self) -> bool {
        self.stored.unwrap_or(self.shown_by_default)
    }

    pub fn changed(&self) -> bool {
        self.shown() != self.shown_by_default
    }

    pub fn shown_by_default(&self) -> bool {
        self.shown_by_default
    }

    /// What to persist: choosing the default stores nothing.
    pub fn stored(&self) -> Option<bool> {
        self.stored
    }

    pub fn set(&mut self, shown: bool) {
        self.stored = if shown == self.shown_by_default {
            None
        } else {
            Some(shown)
        };
    }

    pub fn reset(&mut self) {
        self.stored = None;
    }
}

#[cfg(test)]
#[path = "build_info_tests.rs"]
mod tests;
