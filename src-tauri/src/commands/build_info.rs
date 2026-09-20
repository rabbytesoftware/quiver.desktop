//! The release tag this binary was built from -- when it was built from one.
//!
//! `build.rs` runs at COMPILE time and, if that build is a genuine release
//! build, emits `cargo:rustc-env=QUIVER_DESKTOP_BUILD_TAG=stable-<series>`.
//! `option_env!` then bakes it in as a compile-time constant: `Some(tag)` in a
//! release build, `None` in every dev, PR-CI and untagged build. Nothing here
//! reads the environment at RUNTIME -- a tag on the user's machine, or an
//! exported variable in whatever shell launched the app, cannot influence what
//! this app claims to be.
//!
//! This is what lets `announceSelf` (src/lib/core-store/listeners/
//! self-announce.ts) announce `github.com/rabbytesoftware/quiver.desktop@<tag>`
//! -- a concrete, resolvable ref naming the exact build the user is running --
//! instead of falling back to a refless announce that tracks whatever the
//! latest published release happens to be. See that file for the trade-off the
//! refless path still carries for untagged builds.
//!
//! `tauri.conf.json`'s `version` ("0.1.0") is deliberately NOT what this
//! reports. That is a productVersion, not a git ref: no ref named `0.1.0` will
//! ever exist, because this repo's `stable-release.yml` tags releases
//! `stable-<series>[.patch]`. Announcing it 404'd permanently, which is the
//! defect this module exists to close properly.

/// Whatever `build.rs` baked in, unvalidated.
///
/// `option_env!` is a macro over the COMPILER's environment, so this is a
/// constant in the binary, not a lookup.
const BAKED_TAG: Option<&str> = option_env!("QUIVER_DESKTOP_BUILD_TAG");

/// Does this string name a real quiver.desktop release tag?
///
/// The shape is not a guess: `stable-release.yml`'s `Derive stable tag` step
/// produces exactly `stable-${SERIES}` (e.g. `stable-26.5`) or
/// `stable-${BASE}.$((PATCH + 1))` (e.g. `stable-26.5.1`), and nothing else.
///
/// Validating HERE rather than in `build.rs` is deliberate: `build.rs` cannot
/// be unit-tested (cargo compiles it as its own crate and runs it, it has no
/// test harness), so it stays dumb -- it forwards a candidate and this side,
/// which `cargo test` does reach, decides. It also means a stray local git tag
/// (`wip`, `v2`, a vendor's tag on a fork) or a mistyped CI variable can never
/// make a build claim to be a release: it is rejected and the app falls back to
/// the refless announce, which is always safe.
fn is_release_tag(candidate: &str) -> bool {
	let Some(series) = candidate.strip_prefix("stable-") else {
		return false;
	};
	let parts: Vec<&str> = series.split('.').collect();
	// `stable-<major>.<minor>` or `stable-<major>.<minor>.<patch>`.
	matches!(parts.len(), 2 | 3)
		&& parts.iter()
			.all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The pure core of [`release_tag`], taking the baked-in value as an argument.
///
/// Split out purely so both branches -- built from a tag, and not -- are
/// reachable from a test. `release_tag` itself can only ever exercise the one
/// branch the test binary was compiled with (always `None` under `cargo test`,
/// since a test run is not a release build).
fn resolve(baked: Option<&str>) -> Option<&str> {
	let tag = baked?.trim();
	is_release_tag(tag).then_some(tag)
}

/// The release tag this binary was built from, or `None` for any build that
/// was not cut from one.
pub fn release_tag() -> Option<&'static str> {
	resolve(BAKED_TAG)
}

/// Hands the frontend the tag, or `null`.
///
/// `Option<String>` rather than `String`: absence is the ordinary case (every
/// dev and CI build), not an error, and the frontend branches on it rather
/// than treating an empty string as a sentinel.
#[tauri::command]
pub fn get_build_tag() -> Option<String> {
	release_tag().map(str::to_owned)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_untagged_build_reports_no_tag() {
		assert_eq!(resolve(None), None);
	}

	#[test]
	fn a_release_build_reports_the_tag_it_was_cut_from() {
		assert_eq!(resolve(Some("stable-26.5")), Some("stable-26.5"));
		assert_eq!(resolve(Some("stable-26.5.1")), Some("stable-26.5.1"));
		assert_eq!(resolve(Some("stable-0.1.12")), Some("stable-0.1.12"));
	}

	/// `git describe` ends its output with a newline, and a CI variable can
	/// pick up stray whitespace. Neither should cost a release its stamp.
	#[test]
	fn surrounding_whitespace_is_not_part_of_the_tag() {
		assert_eq!(resolve(Some("  stable-26.5\n")), Some("stable-26.5"));
	}

	#[test]
	fn an_empty_value_is_the_same_as_no_value() {
		assert_eq!(resolve(Some("")), None);
		assert_eq!(resolve(Some("   ")), None);
	}

	/// The original defect: `tauri.conf.json`'s productVersion is not a ref,
	/// and announcing it demanded a git tag named `0.1.0` that nothing ever
	/// creates. It must never be mistaken for a release tag.
	#[test]
	fn the_product_version_is_not_a_release_tag() {
		assert_eq!(resolve(Some("0.1.0")), None);
		assert_eq!(resolve(Some("v0.1.0")), None);
	}

	/// The whole point of `--exact-match`. A plain `git describe` on a commit
	/// three past a tag answers `stable-26.5-3-gabc1234`; if that ever reached
	/// here, a dev build would announce a ref that does not name its own
	/// commit. Belt to `build.rs`'s braces.
	#[test]
	fn a_commit_past_a_tag_is_not_a_release_tag() {
		assert_eq!(resolve(Some("stable-26.5-3-gabc1234")), None);
		assert_eq!(resolve(Some("stable-26.5.1-1-gdeadbee")), None);
	}

	/// This repo tags pre-releases too, and they are not stable releases:
	/// `prerelease.yml` cuts `beta-*`/`hotfix-*`, `nightly.yml` cuts a rolling
	/// `nightly-*`. A rolling tag in particular must never be stamped -- it
	/// moves, so the ref would stop naming this build.
	#[test]
	fn only_stable_tags_count() {
		for other in [
			"beta-26.6",
			"beta-26.6-2",
			"hotfix-26.5.1",
			"nightly-latest",
			"develop",
			"master",
		] {
			assert_eq!(resolve(Some(other)), None, "{other} must not stamp");
		}
	}

	#[test]
	fn a_malformed_stable_tag_is_rejected() {
		for malformed in [
			"stable-",
			"stable-26",
			"stable-26.5.1.2",
			"stable-26.x",
			"stable-26..5",
			"stable-26.5 rc1",
			"Stable-26.5",
			"stable-26.5.1a",
		] {
			assert_eq!(resolve(Some(malformed)), None, "{malformed} must not stamp");
		}
	}

	/// The command is the frontend's only view of this, so it has to agree
	/// with the function it wraps rather than re-deriving anything.
	#[test]
	fn the_command_reports_exactly_what_this_binary_was_built_from() {
		assert_eq!(get_build_tag().as_deref(), release_tag());
	}

	/// A test binary is not a release build, so nothing should have stamped
	/// it. This is what would catch `build.rs` stamping unconditionally -- the
	/// failure mode where every dev build starts claiming to be a release.
	#[test]
	fn this_test_build_is_not_a_release_build() {
		assert_eq!(
			release_tag(),
			None,
			"a `cargo test` build is not cut from a release tag; if this fails, \
			 build.rs stamped a build it should not have (or QUIVER_DESKTOP_RELEASE_TAG \
			 is set in this shell)"
		);
	}
}
