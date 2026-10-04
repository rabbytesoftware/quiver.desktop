use std::path::PathBuf;
use std::process::Command;

/// The variable a release build passes the tag in through.
///
/// `.github/workflows/stable-release.yml` is the only workflow in this repo
/// that creates and pushes its tag BEFORE the build runs (its `prepare` job
/// does; `build` `needs: prepare`), so it is the only one that can name the tag
/// a build is being cut from. `.github/actions/build-tauri` takes it as its
/// `release-tag` input and exports it onto the `cargo tauri build` steps.
const TAG_ENV: &str = "QUIVER_DESKTOP_RELEASE_TAG";

/// The variable the compiled crate reads back through `option_env!`.
///
/// Deliberately a DIFFERENT name from [`TAG_ENV`]: if they were the same,
/// `option_env!` would also see the variable when it happened to be exported in
/// whatever shell ran `cargo build`, bypassing this script entirely and
/// stamping builds nobody meant to stamp.
const BAKED_ENV: &str = "QUIVER_DESKTOP_BUILD_TAG";

/// Bakes the release tag this build is being cut from into the binary, when
/// there is one.
///
/// Emits nothing at all for an ordinary build, so `option_env!(BAKED_ENV)` in
/// `commands::build_info` reads `None` and the app falls back to announcing its
/// self-arrow refless. See that module, and
/// `src/lib/core-store/listeners/self-announce.ts`, for why that matters.
///
/// This script does NOT decide what a valid tag looks like. Cargo compiles a
/// build script as its own crate and runs it; there is no test harness that
/// reaches it, so validation lives in `commands::build_info::is_release_tag`,
/// which `cargo test` does cover. Whatever is forwarded here is a CANDIDATE,
/// and a candidate that fails that check simply never becomes a stamp.
fn main() {
	if let Some(tag) = release_tag_candidate() {
		println!("cargo:rustc-env={BAKED_ENV}={tag}");
	}
	emit_build_stamps();

	// Without this, a cached `target/` (CI restores one; see the `Cache Cargo
	// dependencies` step in .github/actions/build-tauri) could carry a previous
	// release's stamp into the next one: nothing else about the source
	// changes between two consecutive `stable-*` builds.
	println!("cargo:rerun-if-env-changed={TAG_ENV}");
	for var in [COMMIT_ENV, LABEL_ENV, "SOURCE_DATE_EPOCH", "GITHUB_SHA"] {
		println!("cargo:rerun-if-env-changed={var}");
	}
	println!("cargo:rerun-if-changed=build.rs");
	track_git_head();

	tauri_build::build()
}

/// What a workflow passes in so the console indicator can name this build:
/// the commit it was cut from, when the checkout cannot say (a shallow CI
/// checkout can, but a source tarball cannot).
const COMMIT_ENV: &str = "QUIVER_DESKTOP_STAMP_COMMIT";

/// The release tag of ANY channel this build is published as
/// (`stable-26.5.1`, `beta-26.5-2`, `hotfix-26.5.1-1`). Display only: unlike
/// [`TAG_ENV`] it is never announced to quiver.core as a ref, so it does not
/// have to name something a daemon can resolve and it is not limited to
/// `stable-*`.
const LABEL_ENV: &str = "QUIVER_DESKTOP_STAMP_LABEL";

/// The three stamps the build indicator shows, baked in under names the
/// shell cannot reach (the same reasoning as `BAKED_ENV`). Every one is a
/// CANDIDATE: `commands::build_info` validates them at runtime, where
/// `cargo test` reaches, so a malformed value degrades to "unstamped".
///
/// `built_at` is when this script last ran, which is the build time for a
/// release build (CI starts from a clean checkout and sets
/// `SOURCE_DATE_EPOCH` for reproducibility). For an incremental local build it
/// is the time of the last build that re-ran this script -- a commit or tag
/// change -- which is what a dev build wants to report anyway.
fn emit_build_stamps() {
	if let Some(commit) = commit_candidate() {
		println!("cargo:rustc-env=QUIVER_DESKTOP_BUILD_COMMIT={commit}");
	}
	if let Some(epoch) = built_at_epoch() {
		println!("cargo:rustc-env=QUIVER_DESKTOP_BUILD_EPOCH={epoch}");
	}
	if let Some(label) = env_nonempty(LABEL_ENV) {
		println!("cargo:rustc-env=QUIVER_DESKTOP_BUILD_LABEL={label}");
	}
}

fn env_nonempty(name: &str) -> Option<String> {
	let raw = std::env::var(name).ok()?;
	let trimmed = raw.trim().to_owned();
	(!trimmed.is_empty()).then_some(trimmed)
}

/// Workflow-named commit first (`QUIVER_DESKTOP_STAMP_COMMIT`, then
/// `GITHUB_SHA`), else the checkout's own HEAD.
fn commit_candidate() -> Option<String> {
	env_nonempty(COMMIT_ENV)
		.or_else(|| env_nonempty("GITHUB_SHA"))
		.or_else(|| {
			let out = Command::new("git")
				.args(["rev-parse", "HEAD"])
				.output()
				.ok()?;
			if !out.status.success() {
				return None;
			}
			let sha = String::from_utf8(out.stdout).ok()?.trim().to_owned();
			(!sha.is_empty()).then_some(sha)
		})
}

/// `SOURCE_DATE_EPOCH` (the reproducible-builds convention) else now.
fn built_at_epoch() -> Option<u64> {
	if let Some(raw) = env_nonempty("SOURCE_DATE_EPOCH") {
		return raw.parse().ok();
	}
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.ok()
		.map(|d| d.as_secs())
}

/// The tag this build claims, from the workflow if it named one, else from git.
fn release_tag_candidate() -> Option<String> {
	env_tag().or_else(git_exact_tag)
}

/// What the release workflow passed in, if anything.
///
/// First, and not merely as a convenience: the `build` job checks out at
/// `fetch-depth: 1` and never runs `git fetch --tags`, so the tag its own
/// `prepare` job pushed minutes earlier is not in that checkout at all.
/// `git describe` alone would find nothing there and every real release would
/// silently ship unstamped.
fn env_tag() -> Option<String> {
	let raw = std::env::var(TAG_ENV).ok()?;
	let trimmed = raw.trim().to_owned();
	(!trimmed.is_empty()).then_some(trimmed)
}

/// The tag pointing at HEAD, for a build made from a checkout that has one.
///
/// `--exact-match` is the whole point and is not interchangeable with a plain
/// `git describe`: the latter answers `stable-26.5-3-gabc1234` for a commit
/// three past a tag, which is not a ref anyone can resolve and not this build.
/// With it, a commit that is not itself tagged exits non-zero and stamps
/// nothing.
///
/// Absent git, absent repository, shallow clone, no tags: all the same answer,
/// `None`. A build that cannot prove it is a release is not one.
fn git_exact_tag() -> Option<String> {
	let out = Command::new("git")
		.args(["describe", "--tags", "--exact-match", "HEAD"])
		.output()
		.ok()?;
	if !out.status.success() {
		return None;
	}
	let tag = String::from_utf8(out.stdout).ok()?.trim().to_owned();
	(!tag.is_empty()).then_some(tag)
}

/// Re-run this script when the commit or the tag set moves.
///
/// Emitting any `rerun-if-changed` replaces cargo's default "re-run when any
/// file in the package changed" (`tauri_build` already emits its own, so that
/// default is off here regardless). Without these, `git tag stable-x.y &&
/// cargo build` would reuse a cached run and stamp nothing -- the local
/// equivalent of the CI staleness the env var above guards against.
///
/// Only paths that exist are emitted: cargo treats a missing `rerun-if-changed`
/// path as permanently dirty, which would re-run this script -- and so rebuild
/// the whole crate -- on every single build.
fn track_git_head() {
	// A worktree has its own HEAD but shares refs/tags with the main
	// checkout, so the two directories are asked for separately.
	if let Some(git_dir) = git_path("--git-dir") {
		emit_if_present(git_dir.join("HEAD"));
	}
	if let Some(common) = git_path("--git-common-dir") {
		emit_if_present(common.join("packed-refs"));
		// A directory: cargo watches its own mtime, which changes when a tag
		// file is added or removed. Nested tag namespaces (`refs/tags/a/b`)
		// are not covered, and this repo's tags are all flat.
		emit_if_present(common.join("refs").join("tags"));
	}
}

fn git_path(flag: &str) -> Option<PathBuf> {
	let out = Command::new("git")
		.args(["rev-parse", flag])
		.output()
		.ok()?;
	if !out.status.success() {
		return None;
	}
	let path = String::from_utf8(out.stdout).ok()?.trim().to_owned();
	if path.is_empty() {
		return None;
	}
	// `rev-parse` answers relative to the working directory, which for a build
	// script is the package root.
	std::fs::canonicalize(path).ok()
}

fn emit_if_present(path: PathBuf) {
	if path.exists() {
		println!("cargo:rerun-if-changed={}", path.display());
	}
}
