use serde::Serialize;

/// What `build.rs` baked in for the build indicator, unvalidated.
const BAKED_COMMIT: Option<&str> = option_env!("QUIVER_DESKTOP_BUILD_COMMIT");
const BAKED_EPOCH: Option<&str> = option_env!("QUIVER_DESKTOP_BUILD_EPOCH");
const BAKED_LABEL: Option<&str> = option_env!("QUIVER_DESKTOP_BUILD_LABEL");

/// Everything the build indicator shows about THIS binary, beyond the channel
/// (a frontend build constant, `VITE_QUIVER_BUILD_CHANNEL`).
///
/// Every field is `None` for a build that was not stamped, and the frontend
/// renders that as `dev`. Nothing here is read at runtime: a stamp on the
/// user's machine, or a variable exported in the shell that launched the app,
/// cannot change what this app claims to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildStamp {
    /// The full 40-character commit, lower case.
    pub commit: Option<String>,
    /// Unix seconds. The webview formats it; sending a number keeps Rust free
    /// of a date formatter and the frontend free of a parser.
    pub built_at: Option<u64>,
    /// The release tag this build is published as, in any channel
    /// (`stable-26.5.1`, `beta-26.5-2`). `None` for a rolling or dev build.
    pub label: Option<String>,
}

/// A full git object name: exactly 40 lower-case hex digits.
fn valid_commit(candidate: &str) -> Option<&str> {
    let c = candidate.trim();
    (c.len() == 40
        && c.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    .then_some(c)
}

/// A release label this app knows how to display: `stable-<major>.<minor>[.<patch>]`,
/// `beta-<series>[-<n>]` or `hotfix-<series>[-<n>]`, where a series is dotted
/// digits or a `YYYY-MM-DD` date. Anything else (a rolling tag, a branch name,
/// a stray variable) is dropped rather than shown.
fn valid_label(candidate: &str) -> Option<&str> {
    let label = candidate.trim();
    let (channel, rest) = label.split_once('-')?;
    if !matches!(channel, "stable" | "beta" | "hotfix") || rest.is_empty() {
        return None;
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let dotted = |s: &str| s.split('.').all(digits);
    let ok = if channel == "stable" {
        let parts = rest.split('.').count();
        (2..=3).contains(&parts) && dotted(rest)
    } else {
        // `<series>` or `<series>-<n>`, the series dotted digits or a date.
        let date = |s: &str| {
            let p: Vec<&str> = s.split('-').collect();
            p.len() == 3 && p.iter().all(|x| digits(x))
        };
        let parts: Vec<&str> = rest.split('-').collect();
        match parts.len() {
            1 => dotted(parts[0]),
            2 => dotted(parts[0]) && digits(parts[1]),
            3 => date(rest),
            4 => date(&parts[..3].join("-")) && digits(parts[3]),
            _ => false,
        }
    };
    ok.then_some(label)
}

/// The pure core of [`build_stamp`], taking the baked values as arguments so
/// every branch is reachable from a test.
fn resolve_stamp(
    commit: Option<&str>,
    epoch: Option<&str>,
    label: Option<&str>,
) -> BuildStamp {
    BuildStamp {
        commit: commit.and_then(valid_commit).map(str::to_owned),
        built_at: epoch.and_then(|e| e.trim().parse().ok()),
        label: label.and_then(valid_label).map(str::to_owned),
    }
}

pub fn build_stamp() -> BuildStamp {
    resolve_stamp(BAKED_COMMIT, BAKED_EPOCH, BAKED_LABEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "9dd0b183177a64ec71a2672d1cd7cf0c70bb4877";

    #[test]
    fn a_full_lower_case_commit_is_kept() {
        assert_eq!(
            resolve_stamp(Some(SHA), None, None).commit.as_deref(),
            Some(SHA)
        );
    }

    #[test]
    fn a_malformed_commit_is_dropped() {
        for bad in [
            "",
            "9dd0b18",
            &SHA.to_uppercase(),
            &format!("{SHA}0"),
            "zzzz0183177a64ec71a2672d1cd7cf0c70bb4877",
        ] {
            assert_eq!(resolve_stamp(Some(bad), None, None).commit, None, "{bad:?}");
        }
    }

    #[test]
    fn the_build_time_is_unix_seconds_or_nothing() {
        assert_eq!(
            resolve_stamp(None, Some("1791136800"), None).built_at,
            Some(1_791_136_800)
        );
        assert_eq!(resolve_stamp(None, Some(" 12 "), None).built_at, Some(12));
        for bad in ["", "soon", "-1", "1.5", "2026-10-04"] {
            assert_eq!(
                resolve_stamp(None, Some(bad), None).built_at,
                None,
                "{bad:?}"
            );
        }
    }

    #[test]
    fn every_published_release_label_is_kept() {
        for ok in [
            "stable-26.5",
            "stable-26.5.1",
            "beta-26.5",
            "beta-26.5-4",
            "beta-2026-09-27",
            "beta-2026-09-27-1",
            "hotfix-26.5.1",
            "hotfix-26.5.1-2",
        ] {
            assert_eq!(
                resolve_stamp(None, None, Some(ok)).label.as_deref(),
                Some(ok),
                "{ok}"
            );
        }
    }

    #[test]
    fn a_label_that_is_not_a_release_is_dropped() {
        for bad in [
            "",
            "nightly-rolling",
            "nightly-latest",
            "develop",
            "stable-",
            "stable-26",
            "stable-26.5.1.2",
            "beta-",
            "beta-26.5-",
            "beta-26.5-x",
            "beta-26.5-1-2",
            "Beta-26.5",
            "v26.5",
        ] {
            assert_eq!(resolve_stamp(None, None, Some(bad)).label, None, "{bad:?}");
        }
    }

    /// A `cargo test` build is not a release build, so it carries no label --
    /// the failure mode this catches is `build.rs` stamping one unconditionally.
    #[test]
    fn this_test_build_carries_no_release_label() {
        assert_eq!(build_stamp().label, None);
    }
}
