//! Picking and reading quiver.core release tags (port of `resolve-core-version.mjs` and
//! `extract-core-version.mjs`). Pure functions here; the `gh` calls live in `gh.rs`.

use anyhow::{anyhow, bail, Result};

const NIGHTLY_TAG: &str = "nightly-latest";

/// The four numbers a release version ranks by, in the order quiver.core's own channel ranking
/// (and its release-tag.sh) uses: a calendar version `26.5.1` is 26.5.1.0, a date `2026-09-27.1` is
/// 26.9.27.1. `None` for anything else, including a date that is not a real one.
fn rank_key(version: &str) -> Option<[u32; 4]> {
    if let Some(key) = date_key(version) {
        return key;
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() > 4
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let mut key = [0u32; 4];
    for (slot, part) in key.iter_mut().zip(&parts) {
        *slot = part.parse().ok()?;
    }
    Some(key)
}

/// `Some(None)` = it is date-shaped but not a real date; `None` = not date-shaped at all.
fn date_key(version: &str) -> Option<Option<[u32; 4]>> {
    let (date, patch) = match version.split_once('.') {
        Some((d, p)) => (d, Some(p)),
        None => (version, None),
    };
    let fields: Vec<&str> = date.split('-').collect();
    let shaped = fields.len() == 3
        && fields[0].len() == 4
        && fields[1].len() == 2
        && fields[2].len() == 2
        && fields.iter().all(|f| f.bytes().all(|b| b.is_ascii_digit()))
        && patch.is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if !shaped {
        return None;
    }
    let year: u32 = fields[0].parse().ok()?;
    let month: u32 = fields[1].parse().ok()?;
    let day: u32 = fields[2].parse().ok()?;
    let patch: u32 = patch.map_or(Some(0), |p| p.parse().ok())?;
    if !(2000..=2099).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Some(None);
    }
    Some(Some([year - 2000, month, day, patch]))
}

fn highest(mut ranked: Vec<(String, Vec<u32>)>) -> String {
    ranked.sort_by(|a, b| a.1.cmp(&b.1));
    ranked.pop().map(|(tag, _)| tag).unwrap_or_default()
}

/// Picks the highest release tag on a quiver.core channel. `stable` and `beta` are open ranges: "whatever
/// the channel currently calls newest". `nightly` is a single rolling pointer tag.
pub fn resolve_channel(
    tags: &[String],
    channel: &str,
) -> Result<String> {
    if channel == "nightly" {
        if !tags.iter().any(|t| t == NIGHTLY_TAG) {
            bail!("nightly tag \"{NIGHTLY_TAG}\" not found");
        }
        return Ok(NIGHTLY_TAG.to_string());
    }
    let prefix = match channel {
        "stable" => "stable-",
        "beta" => "beta-",
        _ => bail!("unknown channel \"{channel}\""),
    };
    let candidates: Vec<&String> = tags.iter().filter(|t| t.starts_with(prefix)).collect();
    if candidates.is_empty() {
        bail!("no {channel} release found");
    }
    if channel == "stable" {
        let ranked: Vec<_> = candidates
            .iter()
            .filter_map(|t| rank_key(&t[prefix.len()..]).map(|k| (t.to_string(), k.to_vec())))
            .collect();
        if ranked.is_empty() {
            bail!("no {channel} release found");
        }
        return Ok(highest(ranked));
    }
    // beta tags are `beta-<series>[-<count>]`, the series a calendar version or a date ("beta-26.5-4",
    // "beta-2026-09-27-1"). A tag of any other shape is an error rather than something silently sorted in.
    let mut ranked = Vec::new();
    for tag in candidates {
        let (series, count) = split_beta(&tag[prefix.len()..]).ok_or_else(|| malformed(tag))?;
        let mut key = rank_key(series).ok_or_else(|| malformed(tag))?.to_vec();
        key.push(count);
        ranked.push((tag.to_string(), key));
    }
    Ok(highest(ranked))
}

fn malformed(tag: &str) -> anyhow::Error {
    anyhow!("malformed beta tag \"{tag}\"")
}

/// `26.5-4` -> (`26.5`, 4); `2026-09-27-1` -> (`2026-09-27`, 1); `26.5` -> (`26.5`, 0).
fn split_beta(rest: &str) -> Option<(&str, u32)> {
    if rank_key(rest).is_some() {
        return Some((rest, 0));
    }
    let (series, count) = rest.rsplit_once('-')?;
    if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((series, count.parse().ok()?))
}

const NOTES_MARKER: &str = "Built against quiver.core `";

/// The quiver.core tag a desktop release's notes record in their "Built against quiver.core `...`" line.
pub fn extract_core_version(notes: &str) -> Result<String> {
    let start = notes
        .find(NOTES_MARKER)
        .map(|i| i + NOTES_MARKER.len())
        .ok_or_else(|| {
            anyhow!("no \"Built against quiver.core `...`\" line found in release notes")
        })?;
    let len = notes[start..].find('`').filter(|n| *n > 0).ok_or_else(|| {
        anyhow!("no \"Built against quiver.core `...`\" line found in release notes")
    })?;
    Ok(notes[start..start + len].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }
    fn pick(
        list: &[&str],
        channel: &str,
    ) -> String {
        resolve_channel(&tags(list), channel).unwrap()
    }
    fn err(
        list: &[&str],
        channel: &str,
    ) -> String {
        resolve_channel(&tags(list), channel)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn stable_picks_the_highest_release_with_no_range_restriction() {
        let t = [
            "stable-25.8.0",
            "stable-25.9.0",
            "stable-25.9.2",
            "stable-26.0.0",
            "beta-26.1-1",
        ];
        assert_eq!(pick(&t, "stable"), "stable-26.0.0");
    }

    #[test]
    fn stable_without_a_valid_release_fails() {
        assert!(
            err(&["beta-26.5-4", "hotfix-25.9.3-1"], "stable").contains("no stable release found")
        );
        assert!(err(&["stable-not-a-version", "beta-26.5-1"], "stable")
            .contains("no stable release found"));
    }

    #[test]
    fn beta_picks_the_highest_series_then_the_highest_increment() {
        assert_eq!(
            pick(
                &[
                    "beta-26.5",
                    "beta-26.5-1",
                    "beta-26.5-2",
                    "beta-26.6",
                    "stable-26.5.1"
                ],
                "beta"
            ),
            "beta-26.6"
        );
        assert_eq!(
            pick(&["beta-26.5", "beta-26.5-3", "beta-26.5-1"], "beta"),
            "beta-26.5-3"
        );
    }

    #[test]
    fn beta_compares_numerically_not_lexicographically() {
        assert_eq!(pick(&["beta-26.9", "beta-26.10"], "beta"), "beta-26.10");
        assert_eq!(
            pick(&["beta-26.5-9", "beta-26.5-10"], "beta"),
            "beta-26.5-10"
        );
    }

    #[test]
    fn beta_without_a_release_fails() {
        assert!(err(&["stable-26.5.1"], "beta").contains("no beta release found"));
    }

    #[test]
    fn a_malformed_beta_tag_is_an_error_not_silently_sorted_in() {
        assert!(err(&["beta-26.5-4", "beta-abc", "beta-26.4"], "beta")
            .contains("malformed beta tag \"beta-abc\""));
        assert!(err(&["beta-26.5", "beta-2026-13-01"], "beta")
            .contains("malformed beta tag \"beta-2026-13-01\""));
    }

    #[test]
    fn dated_series_rank_by_their_date_among_calendar_versions() {
        let t = [
            "nightly-latest",
            "beta-2026-09-27",
            "stable-26.5.1",
            "beta-26.5-4",
            "beta-26.5",
        ];
        assert_eq!(pick(&t, "beta"), "beta-2026-09-27");
        assert_eq!(
            pick(
                &["beta-2026-09-27", "beta-2026-09-27-1", "beta-26.5-4"],
                "beta"
            ),
            "beta-2026-09-27-1"
        );
        assert_eq!(
            pick(&["beta-2026-09-27-3", "beta-26.11"], "beta"),
            "beta-26.11"
        );
        assert_eq!(
            pick(&["beta-2026-09-27-3", "beta-2026-10-01"], "beta"),
            "beta-2026-10-01"
        );
    }

    #[test]
    fn stable_dates_rank_as_yy_mm_dd_patch() {
        let t = [
            "stable-26.5.1",
            "stable-2026-09-27",
            "stable-2026-09-27.1",
            "stable-2026-10-01",
        ];
        assert_eq!(pick(&t, "stable"), "stable-2026-10-01");
        let mut more = t.to_vec();
        more.push("stable-26.11");
        assert_eq!(pick(&more, "stable"), "stable-26.11");
        assert_eq!(
            pick(
                &["stable-2026-09-27", "stable-2026-09-27.1", "stable-26.5.1"],
                "stable"
            ),
            "stable-2026-09-27.1"
        );
    }

    #[test]
    fn a_stable_tag_with_an_impossible_date_is_ignored() {
        assert_eq!(
            pick(&["stable-26.5", "stable-2026-02-40"], "stable"),
            "stable-26.5"
        );
    }

    #[test]
    fn nightly_is_the_rolling_tag_when_it_exists() {
        assert_eq!(
            pick(
                &["stable-26.5.1", "nightly-latest", "beta-26.5-4"],
                "nightly"
            ),
            "nightly-latest"
        );
        assert!(err(&["stable-26.5.1"], "nightly").contains("not found"));
    }

    #[test]
    fn an_unknown_channel_is_refused() {
        assert!(err(&["stable-26.5.1"], "hotfix").contains("unknown channel \"hotfix\""));
    }

    #[test]
    fn the_core_version_is_read_from_the_notes_footer() {
        let notes = "## Changes since stable-26.5.0\n- fix: something (Mateo)\n\nBuilt against quiver.core `stable-26.5.1`.\n";
        assert_eq!(extract_core_version(notes).unwrap(), "stable-26.5.1");
    }

    #[test]
    fn notes_without_a_core_version_line_fail() {
        assert!(extract_core_version("## Initial Release\n")
            .unwrap_err()
            .to_string()
            .contains("no \"Built against"));
    }
}
