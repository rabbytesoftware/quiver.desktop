//! Resolving THIS app's own release asset out of the GitHub releases API.
//!
//! WHY THIS EXISTS. `ARROW.md`'s `install` and `update` lifecycles fetch
//! `${QUIVER_RELEASE_ASSET_URL}` and verify `${QUIVER_RELEASE_CHECKSUM}`, and
//! neither variable has a default, so quiver.core requires both from whoever
//! starts the execution. The manifest cannot supply them itself: release
//! filenames carry `tauri.conf.json`'s static `"0.1.0"`, which never tracks
//! the git tag a release is cut from, so no name is derivable from anything
//! the manifest knows -- and `${REF}` during an update is the ref being
//! updated FROM, not the one being updated TO, so templating from it would
//! re-download the version the user already has. The contract is therefore
//! that the CALLER resolves the asset. `install.sh` is the caller when a
//! human runs the one-liner; this module is the caller when the user clicks
//! Update inside the app.
//!
//! WHY RUST AND NOT THE FRONTEND. Selection needs this machine's real OS and
//! CPU architecture, and the webview cannot tell the truth about either:
//! `navigator.platform` answers `MacIntel` on Apple Silicon, and no WebKitGTK
//! user-agent string distinguishes amd64 from arm64. Picking the wrong
//! architecture here installs a bundle that cannot run. `std::env::consts`
//! is the compiler's own answer and is the direct analogue of `install.sh`'s
//! `uname -s` / `uname -m`. Doing the request here also keeps it off the
//! webview's network stack, where it would be the only cross-origin fetch the
//! app ever makes.
//!
//! The selection rules below are a port of `install.sh`'s, not a second
//! opinion: pick by extension (never by filename), narrow by architecture only
//! when that narrowing leaves something, and take the first survivor.

use serde::{Deserialize, Serialize};

/// The repository this app publishes itself from.
pub const DEFAULT_REPO: &str = "rabbytesoftware/quiver.desktop";

/// GitHub's API origin. Overridable only so the tests can point at a local
/// server; nothing reads an environment variable for it at runtime.
pub const DEFAULT_API: &str = "https://api.github.com";

/// GitHub rejects an API request that arrives without one.
const USER_AGENT: &str = concat!("quiver.desktop/", env!("CARGO_PKG_VERSION"));

const REQUEST_TIMEOUT_SECS: u64 = 15;

/// One asset as the releases API reports it.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiAsset {
	pub name: String,
	pub browser_download_url: String,
	/// `"sha256:<hex>"` when GitHub has one for this asset, absent or null
	/// otherwise. GitHub began reporting this in 2025 and it is populated for
	/// every asset uploaded since; an older release predating the feature
	/// carries `null`, which is why [`resolve_checksum`] still falls back to a
	/// published checksum manifest.
	#[serde(default)]
	pub digest: Option<String>,
}

/// The subset of a release document this module reads.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiRelease {
	#[serde(default)]
	pub tag_name: String,
	#[serde(default)]
	pub assets: Vec<ApiAsset>,
}

/// What the frontend gets back: enough to start an execution with.
///
/// `checksum` is an `Option` on purpose. This type reports what the release
/// actually publishes; whether an unverifiable asset may be installed is a
/// policy decision, and it is made at the call site (see
/// `src/features/arrow-details/lib/release-variables.ts`) rather than buried
/// in a resolver that would have to decide it identically for every caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedAsset {
	/// The release tag the asset came from, for display.
	pub tag: String,
	/// The asset's filename, for display and for the checksum-manifest lookup.
	pub name: String,
	/// `QUIVER_RELEASE_ASSET_URL`.
	pub url: String,
	/// `QUIVER_RELEASE_CHECKSUM`: bare lowercase hex, no algorithm prefix,
	/// which is the only form `verifyChecksum` in quiver.core's download step
	/// handler compares against. `None` when this release publishes no digest
	/// and no checksum manifest.
	pub checksum: Option<String>,
}

/// Why a resolution failed, in terms the UI can turn into a sentence.
///
/// `kind` is a stable machine-readable tag the frontend maps to a translated
/// message; `detail` is the technical text, shown only where technical text
/// already belongs (the same problem dialog that shows a failed step's own
/// error verbatim).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolveError {
	pub kind: &'static str,
	pub detail: String,
}

impl ResolveError {
	fn new(kind: &'static str, detail: impl Into<String>) -> Self {
		ResolveError {
			kind,
			detail: detail.into(),
		}
	}
}

/// The machine could not reach the API at all: DNS, TLS, connect or timeout.
pub const KIND_OFFLINE: &str = "offline";
/// GitHub answered, and refused on quota. Unauthenticated callers get 60
/// requests an hour per IP, which a shared NAT can exhaust without this user
/// having made a single one.
pub const KIND_RATE_LIMITED: &str = "rate_limited";
/// The repository publishes no release the API will name.
pub const KIND_NO_RELEASE: &str = "no_release";
/// Any other non-success status.
pub const KIND_HTTP: &str = "http";
/// A 2xx whose body was not a release document.
pub const KIND_MALFORMED: &str = "malformed";
/// The release exists but carries nothing installable on this platform.
pub const KIND_NO_ASSET: &str = "no_asset";
/// This build is running somewhere the manifest has no bundle for.
pub const KIND_UNSUPPORTED_PLATFORM: &str = "unsupported_platform";

/// The filename suffix identifying this platform's bundle.
///
/// These are `ARROW.md`'s own three choices, and `install.sh`'s and
/// `install.ps1`'s: the AppImage on Linux (the `.deb` published beside it
/// needs root and a Debian-family distro), the `.dmg` on macOS, and the NSIS
/// `*-setup.exe` on Windows (the `.msi` published beside it installs
/// per-machine and would need elevation). Diverging here would mean the app
/// updating itself into a different place than it installed itself.
fn platform_suffix(os: &str) -> Option<&'static str> {
	match os {
		"linux" => Some(".appimage"),
		"macos" => Some(".dmg"),
		"windows" => Some("-setup.exe"),
		_ => None,
	}
}

/// Every spelling of an architecture that appears in a bundle filename.
///
/// Tauri names the Linux bundle with the Debian architecture (`amd64`) and the
/// macOS one with the Rust triple's (`x86_64`); `x64` and `aarch64` are what
/// other tooling emits. An unrecognised architecture returns an empty slice,
/// which [`select_asset`] reads as "do not narrow" -- so a release carrying
/// exactly one asset of the right kind still resolves, exactly as `install.sh`
/// treats its `unknown`.
fn arch_aliases(arch: &str) -> &'static [&'static str] {
	match arch {
		"x86_64" => &["amd64", "x86_64", "x64"],
		"aarch64" => &["arm64", "aarch64"],
		_ => &[],
	}
}

fn names_arch(name: &str, aliases: &[&str]) -> bool {
	let lower = name.to_ascii_lowercase();
	aliases.iter().any(|alias| lower.contains(alias))
}

/// Picks the asset to install out of a release's asset list.
///
/// Selection is by extension, never by filename: quiver.desktop's release
/// filenames carry a product version that does not track the git tag, so no
/// name can be predicted from the tag being installed.
///
/// Architecture narrows the candidates only when it leaves at least one --
/// a release publishing a single universal bundle (which is what the macOS
/// `.dmg` is) must not be narrowed out of existence.
pub fn select_asset<'a>(
	assets: &'a [ApiAsset],
	os: &str,
	arch: &str,
) -> Result<&'a ApiAsset, ResolveError> {
	let Some(suffix) = platform_suffix(os) else {
		return Err(ResolveError::new(
			KIND_UNSUPPORTED_PLATFORM,
			format!("no Quiver bundle is published for {os}"),
		));
	};

	let candidates: Vec<&ApiAsset> = assets
		.iter()
		.filter(|asset| asset.name.to_ascii_lowercase().ends_with(suffix))
		.collect();

	if candidates.is_empty() {
		return Err(ResolveError::new(
			KIND_NO_ASSET,
			format!("this release publishes no *{suffix} asset for {os}/{arch}"),
		));
	}

	let aliases = arch_aliases(arch);
	let narrowed: Vec<&ApiAsset> = candidates
		.iter()
		.copied()
		.filter(|asset| names_arch(&asset.name, aliases))
		.collect();

	let chosen = if narrowed.is_empty() {
		candidates[0]
	} else {
		narrowed[0]
	};
	Ok(chosen)
}

/// The URL of the release's sha256sum-format checksum manifest, when it
/// publishes one.
///
/// Matched against the same four names `install.sh` and `install.ps1` accept.
pub fn checksum_manifest_url(assets: &[ApiAsset]) -> Option<&str> {
	assets.iter()
		.find(|asset| {
			matches!(
				asset.name.to_ascii_lowercase().as_str(),
				"checksums" | "checksums.txt" | "sha256sums" | "sha256sums.txt"
			)
		})
		.map(|asset| asset.browser_download_url.as_str())
}

/// The bare hex out of an API `digest` value.
///
/// The API writes `sha256:<hex>`; quiver.core's fetch step compares a BARE
/// hex digest with no algorithm prefix, so the prefix has to come off here. A
/// digest under any other algorithm, or one that is not hex, is discarded
/// rather than passed on as something that would fail verification with a
/// misleading "checksum mismatch".
pub fn digest_hex(digest: &str) -> Option<String> {
	let hex = digest.strip_prefix("sha256:")?;
	if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
		return None;
	}
	Some(hex.to_ascii_lowercase())
}

/// The digest recorded for `name` in a sha256sum-format manifest.
///
/// Handles both forms `sha256sum` writes: `<hash>  name` and `<hash> *name`.
pub fn expected_sum(manifest: &str, name: &str) -> Option<String> {
	for line in manifest.lines() {
		let mut fields = line.split_whitespace();
		let (Some(hash), Some(entry)) = (fields.next(), fields.next()) else {
			continue;
		};
		if entry.trim_start_matches('*') == name {
			return Some(hash.to_ascii_lowercase());
		}
	}
	None
}

/// A client configured the way GitHub expects to be called.
fn client() -> Result<reqwest::Client, ResolveError> {
	reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
		.build()
		.map_err(|err| ResolveError::new(KIND_OFFLINE, err.to_string()))
}

fn release_url(api_base: &str, repo: &str, tag: Option<&str>) -> String {
	match tag {
		Some(tag) => format!("{api_base}/repos/{repo}/releases/tags/{tag}"),
		None => format!("{api_base}/repos/{repo}/releases/latest"),
	}
}

/// Turns a non-success status into the kind the UI branches on.
fn status_error(status: reqwest::StatusCode, body: &str) -> ResolveError {
	let kind = match status.as_u16() {
		403 | 429 => KIND_RATE_LIMITED,
		404 => KIND_NO_RELEASE,
		_ => KIND_HTTP,
	};
	ResolveError::new(kind, format!("GitHub answered {status}: {body}"))
}

/// Reads the release document.
async fn fetch_release(
	http: &reqwest::Client,
	api_base: &str,
	repo: &str,
	tag: Option<&str>,
) -> Result<ApiRelease, ResolveError> {
	let url = release_url(api_base, repo, tag);
	let response = http
		.get(&url)
		.header("Accept", "application/vnd.github+json")
		.send()
		.await
		.map_err(|err| ResolveError::new(KIND_OFFLINE, err.to_string()))?;

	let status = response.status();
	let body = response
		.text()
		.await
		.map_err(|err| ResolveError::new(KIND_OFFLINE, err.to_string()))?;

	if !status.is_success() {
		// Truncated: a rate-limit body is a paragraph, and this ends up in a
		// dialog next to a sentence a person has to read.
		let excerpt: String = body.chars().take(200).collect();
		return Err(status_error(status, &excerpt));
	}

	serde_json::from_str::<ApiRelease>(&body)
		.map_err(|err| ResolveError::new(KIND_MALFORMED, err.to_string()))
}

/// The asset's checksum, from the strongest source the release offers.
///
/// Two sources, in order:
///
///   1. the API's own per-asset `digest`, which is GitHub's record of what it
///      stored and needs no extra request;
///   2. a published sha256sum-format manifest, which is what `install.sh`
///      has always read.
///
/// Neither being available is NOT an error here: it is reported as `None` and
/// decided on by the caller. A manifest that cannot be downloaded, or that has
/// no line for this asset, is treated the same as no manifest at all -- the
/// same call `install.sh` makes, for the same reason (the download still came
/// from GitHub over TLS), and the one it is deliberately consistent with.
async fn resolve_checksum(
	http: &reqwest::Client,
	asset: &ApiAsset,
	assets: &[ApiAsset],
) -> Option<String> {
	if let Some(hex) = asset.digest.as_deref().and_then(digest_hex) {
		return Some(hex);
	}

	let manifest_url = checksum_manifest_url(assets)?;
	let manifest = http
		.get(manifest_url)
		.send()
		.await
		.ok()?
		.text()
		.await
		.ok()?;
	expected_sum(&manifest, &asset.name)
}

/// Resolves this machine's asset out of `repo`'s release, against `api_base`.
///
/// Split from [`resolve_latest`] so the tests can drive the whole path,
/// request and all, against a local server.
pub async fn resolve(
	api_base: &str,
	repo: &str,
	tag: Option<&str>,
	os: &str,
	arch: &str,
) -> Result<ResolvedAsset, ResolveError> {
	let http = client()?;
	let release = fetch_release(&http, api_base, repo, tag).await?;
	let asset = select_asset(&release.assets, os, arch)?;
	let checksum = resolve_checksum(&http, asset, &release.assets).await;

	Ok(ResolvedAsset {
		tag: release.tag_name.clone(),
		name: asset.name.clone(),
		url: asset.browser_download_url.clone(),
		checksum,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use wiremock::matchers::{header, method, path};
	use wiremock::{Mock, MockServer, ResponseTemplate};

	fn asset(name: &str, digest: Option<&str>) -> ApiAsset {
		ApiAsset {
			name: name.to_string(),
			browser_download_url: format!(
				"https://github.com/rabbytesoftware/quiver.desktop/releases/download/stable-1.0/{name}"
			),
			digest: digest.map(str::to_string),
		}
	}

	const SUM_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
	const SUM_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

	// ── selection ────────────────────────────────────────────────────────

	/// The whole reason selection is by extension: these are the six names a
	/// real `tauri build` publishes, all carrying the same static product
	/// version, none of them naming the tag.
	fn full_release() -> Vec<ApiAsset> {
		vec![
			asset("quiver-desktop_0.1.0_amd64.AppImage", None),
			asset("quiver-desktop_0.1.0_amd64.deb", None),
			asset("Quiver_0.1.0_aarch64.dmg", None),
			asset("Quiver_0.1.0_x64_en-US.msi", None),
			asset("Quiver_0.1.0_x64-setup.exe", None),
		]
	}

	#[test]
	fn linux_takes_the_appimage_and_never_the_deb() {
		let assets = full_release();
		let chosen = select_asset(&assets, "linux", "x86_64").unwrap();
		assert_eq!(chosen.name, "quiver-desktop_0.1.0_amd64.AppImage");
	}

	#[test]
	fn macos_takes_the_dmg() {
		let assets = full_release();
		let chosen = select_asset(&assets, "macos", "aarch64").unwrap();
		assert_eq!(chosen.name, "Quiver_0.1.0_aarch64.dmg");
	}

	/// The `.msi` is published beside the NSIS bundle and installs
	/// per-machine, so it would need elevation. `ARROW.md` and `install.ps1`
	/// both take the `-setup.exe`; so must this.
	#[test]
	fn windows_takes_the_nsis_setup_and_never_the_msi() {
		let assets = full_release();
		let chosen = select_asset(&assets, "windows", "x86_64").unwrap();
		assert_eq!(chosen.name, "Quiver_0.1.0_x64-setup.exe");
	}

	#[test]
	fn architecture_decides_between_two_bundles_of_the_same_kind() {
		let assets = vec![
			asset("Quiver_0.1.0_x64.dmg", None),
			asset("Quiver_0.1.0_aarch64.dmg", None),
		];
		assert_eq!(
			select_asset(&assets, "macos", "aarch64").unwrap().name,
			"Quiver_0.1.0_aarch64.dmg"
		);
		assert_eq!(
			select_asset(&assets, "macos", "x86_64").unwrap().name,
			"Quiver_0.1.0_x64.dmg"
		);
	}

	/// A universal bundle names no architecture at all. Narrowing must not
	/// remove the only candidate.
	#[test]
	fn a_release_with_one_unnamed_bundle_still_resolves() {
		let assets = vec![asset("Quiver_0.1.0_universal.dmg", None)];
		assert_eq!(
			select_asset(&assets, "macos", "aarch64").unwrap().name,
			"Quiver_0.1.0_universal.dmg"
		);
	}

	/// `install.sh` prints `unknown` for a machine name it does not know and
	/// carries on without narrowing. Same rule here.
	#[test]
	fn an_unknown_architecture_does_not_narrow() {
		let assets = vec![asset("quiver-desktop_0.1.0_amd64.AppImage", None)];
		assert_eq!(
			select_asset(&assets, "linux", "powerpc64").unwrap().name,
			"quiver-desktop_0.1.0_amd64.AppImage"
		);
	}

	#[test]
	fn a_release_with_nothing_for_this_platform_is_an_error() {
		let assets = vec![asset("quiver-desktop_0.1.0_amd64.AppImage", None)];
		let err = select_asset(&assets, "macos", "aarch64").unwrap_err();
		assert_eq!(err.kind, KIND_NO_ASSET);
	}

	#[test]
	fn an_empty_release_is_an_error_not_a_panic() {
		let err = select_asset(&[], "linux", "x86_64").unwrap_err();
		assert_eq!(err.kind, KIND_NO_ASSET);
	}

	#[test]
	fn a_platform_with_no_bundle_is_named_as_such() {
		let assets = full_release();
		let err = select_asset(&assets, "freebsd", "x86_64").unwrap_err();
		assert_eq!(err.kind, KIND_UNSUPPORTED_PLATFORM);
	}

	/// GitHub preserves upload order, and the case of an extension is the
	/// bundler's choice (`.AppImage`), so matching must be case-insensitive
	/// without the match itself reordering anything.
	#[test]
	fn extension_matching_ignores_case() {
		let assets = vec![asset("Quiver_0.1.0_amd64.appimage", None)];
		assert_eq!(
			select_asset(&assets, "linux", "x86_64").unwrap().name,
			"Quiver_0.1.0_amd64.appimage"
		);
	}

	// ── checksums ────────────────────────────────────────────────────────

	#[test]
	fn a_sha256_digest_becomes_bare_hex() {
		assert_eq!(
			digest_hex(&format!("sha256:{}", SUM_A.to_uppercase())),
			Some(SUM_A.to_string())
		);
	}

	/// quiver.core's fetch step compares a bare hex digest and does no
	/// prefix-stripping of its own, so anything this cannot reduce to bare
	/// hex must be dropped rather than forwarded.
	#[test]
	fn a_digest_this_cannot_use_is_dropped() {
		assert_eq!(digest_hex("sha512:abc"), None);
		assert_eq!(digest_hex(SUM_A), None);
		assert_eq!(digest_hex("sha256:"), None);
		assert_eq!(digest_hex("sha256:nothex000000000000000000000000000000000000000000000000000000000"), None);
		assert_eq!(digest_hex(&format!("sha256:{}00", SUM_A)), None);
	}

	/// A manifest is a text file, and text files have blank lines, trailing
	/// newlines and the occasional comment. A line without two fields is
	/// skipped, not read as an entry whose name happens to be missing.
	#[test]
	fn a_manifest_line_that_is_not_an_entry_is_skipped() {
		let manifest = format!("\n# generated by ci\n\n{SUM_A}  Quiver.AppImage\n\n");
		assert_eq!(
			expected_sum(&manifest, "Quiver.AppImage"),
			Some(SUM_A.to_string())
		);
		assert_eq!(expected_sum("\n\n   \n", "Quiver.AppImage"), None);
	}

	#[test]
	fn a_checksum_manifest_is_read_in_both_forms_sha256sum_writes() {
		let manifest = format!("{SUM_A}  Quiver.AppImage\n{SUM_B} *Quiver.dmg\n");
		assert_eq!(
			expected_sum(&manifest, "Quiver.AppImage"),
			Some(SUM_A.to_string())
		);
		assert_eq!(
			expected_sum(&manifest, "Quiver.dmg"),
			Some(SUM_B.to_string())
		);
		assert_eq!(expected_sum(&manifest, "Quiver-setup.exe"), None);
	}

	#[test]
	fn a_checksum_manifest_asset_is_found_under_any_of_its_names() {
		for name in ["checksums.txt", "checksums", "SHA256SUMS", "sha256sums.txt"] {
			let assets = vec![asset("x.AppImage", None), asset(name, None)];
			assert!(
				checksum_manifest_url(&assets).is_some(),
				"{name} should be recognised"
			);
		}
		assert!(checksum_manifest_url(&[asset("x.AppImage", None)]).is_none());
	}

	// ── the whole path, against a real server ────────────────────────────

	async fn serve(release: serde_json::Value) -> MockServer {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/latest",
			))
			.and(header("accept", "application/vnd.github+json"))
			.respond_with(ResponseTemplate::new(200).set_body_json(release))
			.mount(&server)
			.await;
		server
	}

	#[tokio::test]
	async fn the_normal_case_resolves_a_url_and_a_checksum() {
		let server = serve(serde_json::json!({
			"tag_name": "stable-26.9",
			"assets": [
				{
					"name": "quiver-desktop_0.1.0_amd64.AppImage",
					"browser_download_url": "https://github.com/r/q/releases/download/stable-26.9/quiver-desktop_0.1.0_amd64.AppImage",
					"digest": format!("sha256:{SUM_A}")
				}
			]
		}))
		.await;

		let resolved = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap();

		assert_eq!(resolved.tag, "stable-26.9");
		assert_eq!(resolved.name, "quiver-desktop_0.1.0_amd64.AppImage");
		assert!(resolved
			.url
			.ends_with("quiver-desktop_0.1.0_amd64.AppImage"));
		assert_eq!(resolved.checksum.as_deref(), Some(SUM_A));
	}

	/// An asset predating GitHub's digest field still verifies, off the
	/// manifest `install.sh` has always read.
	#[tokio::test]
	async fn a_digestless_asset_falls_back_to_the_published_manifest() {
		let server = MockServer::start().await;
		let base = server.uri();
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/latest",
			))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"tag_name": "stable-26.9",
				"assets": [
					{
						"name": "Quiver.AppImage",
						"browser_download_url": format!("{base}/dl/Quiver.AppImage"),
						"digest": serde_json::Value::Null
					},
					{
						"name": "checksums.txt",
						"browser_download_url": format!("{base}/dl/checksums.txt")
					}
				]
			})))
			.mount(&server)
			.await;
		Mock::given(method("GET"))
			.and(path("/dl/checksums.txt"))
			.respond_with(
				ResponseTemplate::new(200)
					.set_body_string(format!("{SUM_B}  Quiver.AppImage\n")),
			)
			.mount(&server)
			.await;

		let resolved = resolve(&base, DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap();
		assert_eq!(resolved.checksum.as_deref(), Some(SUM_B));
	}

	/// The state every quiver.desktop release is in today:
	/// `stable-release.yml` uploads the bundles and nothing else, and these
	/// assets predate nothing -- the digest is simply absent. Resolution
	/// SUCCEEDS with no checksum; refusing to install is the caller's
	/// decision, not this function's.
	#[tokio::test]
	async fn a_release_with_no_checksum_anywhere_resolves_without_one() {
		let server = serve(serde_json::json!({
			"tag_name": "stable-26.9",
			"assets": [
				{
					"name": "Quiver.AppImage",
					"browser_download_url": "https://github.com/r/q/releases/download/stable-26.9/Quiver.AppImage"
				}
			]
		}))
		.await;

		let resolved = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap();
		assert_eq!(resolved.checksum, None);
	}

	/// A manifest that is published but 404s, or carries no line for this
	/// asset, is the same as no manifest -- never a hard failure, and never a
	/// checksum invented to fill the hole.
	#[tokio::test]
	async fn an_unreadable_checksum_manifest_is_the_same_as_none() {
		let server = MockServer::start().await;
		let base = server.uri();
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/latest",
			))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"tag_name": "stable-26.9",
				"assets": [
					{ "name": "Quiver.AppImage", "browser_download_url": format!("{base}/dl/Quiver.AppImage") },
					{ "name": "checksums.txt", "browser_download_url": format!("{base}/dl/checksums.txt") }
				]
			})))
			.mount(&server)
			.await;
		Mock::given(method("GET"))
			.and(path("/dl/checksums.txt"))
			.respond_with(ResponseTemplate::new(404))
			.mount(&server)
			.await;

		let resolved = resolve(&base, DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap();
		assert_eq!(resolved.checksum, None);
	}

	#[tokio::test]
	async fn no_asset_for_this_platform_is_reported_as_such() {
		let server = serve(serde_json::json!({
			"tag_name": "stable-26.9",
			"assets": [
				{ "name": "quiver-desktop_0.1.0_amd64.deb", "browser_download_url": "https://x/y.deb" }
			]
		}))
		.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_NO_ASSET);
	}

	#[tokio::test]
	async fn a_rate_limited_api_is_distinguished_from_everything_else() {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/latest",
			))
			.respond_with(ResponseTemplate::new(403).set_body_string(
				"{\"message\":\"API rate limit exceeded for 1.2.3.4.\"}",
			))
			.mount(&server)
			.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_RATE_LIMITED);
	}

	/// Everything that is neither a quota refusal nor a missing release: a
	/// 500, a 502 from something in front of GitHub, a 401 from a proxy. The
	/// UI has one sentence for all of them, but it must be THAT sentence and
	/// not "check your connection", which would send the user to fix
	/// something that is not broken.
	#[tokio::test]
	async fn any_other_bad_status_is_its_own_kind() {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/latest",
			))
			.respond_with(
				ResponseTemplate::new(500).set_body_string("upstream is unwell"),
			)
			.mount(&server)
			.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_HTTP);
	}

	#[tokio::test]
	async fn a_repository_with_no_release_is_distinguished_from_a_dead_network() {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/latest",
			))
			.respond_with(
				ResponseTemplate::new(404)
					.set_body_string("{\"message\":\"Not Found\"}"),
			)
			.mount(&server)
			.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_NO_RELEASE);
	}

	/// No server at all. Port 1 is reserved and nothing binds it, so the
	/// connection is refused rather than merely unanswered -- which is the
	/// distinction that matters: a refusal must read as `offline`, not get
	/// mistaken for an HTTP answer. A dropped `MockServer`'s port is NOT a
	/// substitute; it is released back to the OS and can be reused, and this
	/// test caught exactly that by reporting `no_release`.
	#[tokio::test]
	async fn an_unreachable_api_reports_offline() {
		let err = resolve("http://127.0.0.1:1", DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_OFFLINE);
	}

	#[tokio::test]
	async fn a_body_that_is_not_a_release_is_reported_as_malformed() {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/latest",
			))
			.respond_with(
				ResponseTemplate::new(200).set_body_string("<html>nope</html>"),
			)
			.mount(&server)
			.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_MALFORMED);
	}

	#[tokio::test]
	async fn an_explicit_tag_reads_that_tag_rather_than_the_latest() {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(
				"/repos/rabbytesoftware/quiver.desktop/releases/tags/stable-26.8",
			))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"tag_name": "stable-26.8",
				"assets": [
					{ "name": "Quiver.AppImage", "browser_download_url": "https://x/Quiver.AppImage" }
				]
			})))
			.mount(&server)
			.await;

		let resolved = resolve(
			&server.uri(),
			DEFAULT_REPO,
			Some("stable-26.8"),
			"linux",
			"x86_64",
		)
		.await
		.unwrap();
		assert_eq!(resolved.tag, "stable-26.8");
	}
}
