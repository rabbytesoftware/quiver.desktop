//! Resolving THIS app's own release asset from its GitHub release page.
//!
//! `ARROW.md`'s `install`/`update` lifecycles need `${QUIVER_RELEASE_ASSET_URL}`
//! and `${QUIVER_RELEASE_CHECKSUM}` with no default, so the caller must
//! resolve them: the manifest can't, since release filenames carry
//! `tauri.conf.json`'s static "0.1.0", not the git tag, and `${REF}` during
//! an update names the version being left, not the target. `install.sh` is
//! the caller for the one-liner; this module is the caller when the user
//! clicks Update in the app.
//!
//! Done in Rust, not the frontend, because selection needs the real OS/CPU
//! architecture and the webview cannot report either honestly (Apple
//! Silicon's `navigator.platform` still answers `MacIntel`).
//! `std::env::consts` is the direct analogue of `install.sh`'s
//! `uname -s`/`uname -m`.
//!
//! Selection rules below are a port of `install.sh`'s, not a second opinion.
//!
//! Nothing here calls `api.github.com`. Its anonymous quota is 60 requests an
//! hour per IP, which a shared NAT exhausts without this user having made one,
//! and the update then refuses to start. The release page does not count
//! against it: the tag is the redirect of `/releases/latest`, and the assets
//! and their sha256 digests are on `/releases/expanded_assets/<tag>`.

use serde::Serialize;

/// The repository this app publishes itself from.
pub const DEFAULT_REPO: &str = "rabbytesoftware/quiver.desktop";

/// GitHub's origin. Overridable only so the tests can point at a local
/// server; nothing reads an environment variable for it at runtime.
pub const DEFAULT_ORIGIN: &str = "https://github.com";

/// GitHub rejects a request that arrives without one.
const USER_AGENT: &str = concat!("quiver.desktop/", env!("CARGO_PKG_VERSION"));

const REQUEST_TIMEOUT_SECS: u64 = 15;

/// One asset as the release page lists it.
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
	pub name: String,
	pub browser_download_url: String,
	/// `"sha256:<hex>"` when the page shows one for this asset, `None`
	/// otherwise. GitHub began recording it in 2025, so an older release
	/// carries none, which is why [`resolve_checksum`] still falls back to a
	/// published checksum manifest.
	pub digest: Option<String>,
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

/// The machine could not reach GitHub at all: DNS, TLS, connect or timeout.
pub const KIND_OFFLINE: &str = "offline";
/// GitHub answered, and refused on quota: the release page throttles a client
/// that asks too often, with a 403 or 429.
pub const KIND_RATE_LIMITED: &str = "rate_limited";
/// The repository publishes no release GitHub will name.
pub const KIND_NO_RELEASE: &str = "no_release";
/// Any other non-success status.
pub const KIND_HTTP: &str = "http";
/// A 2xx whose body was not a release page.
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
	assets: &'a [ReleaseAsset],
	os: &str,
	arch: &str,
) -> Result<&'a ReleaseAsset, ResolveError> {
	let Some(suffix) = platform_suffix(os) else {
		return Err(ResolveError::new(
			KIND_UNSUPPORTED_PLATFORM,
			format!("no Quiver bundle is published for {os}"),
		));
	};

	let candidates: Vec<&ReleaseAsset> = assets
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
	let narrowed: Vec<&ReleaseAsset> = candidates
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
pub fn checksum_manifest_url(assets: &[ReleaseAsset]) -> Option<&str> {
	assets.iter()
		.find(|asset| {
			matches!(
				asset.name.to_ascii_lowercase().as_str(),
				"checksums" | "checksums.txt" | "sha256sums" | "sha256sums.txt"
			)
		})
		.map(|asset| asset.browser_download_url.as_str())
}

/// The bare hex out of a `digest` value.
///
/// GitHub writes `sha256:<hex>`; quiver.core's fetch step compares a BARE
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

/// A client configured the way GitHub expects to be called. Redirects are not
/// followed: the tag of the latest release is read off the redirect itself.
fn client() -> Result<reqwest::Client, ResolveError> {
	reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
		.redirect(reqwest::redirect::Policy::none())
		.build()
		.map_err(|err| ResolveError::new(KIND_OFFLINE, err.to_string()))
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

/// Reads one page, as text. A non-success status becomes its error kind.
async fn get_page(http: &reqwest::Client, url: &str) -> Result<reqwest::Response, ResolveError> {
	http.get(url)
		.send()
		.await
		.map_err(|err| ResolveError::new(KIND_OFFLINE, err.to_string()))
}

async fn page_body(response: reqwest::Response) -> Result<String, ResolveError> {
	let status = response.status();
	let body = response
		.text()
		.await
		.map_err(|err| ResolveError::new(KIND_OFFLINE, err.to_string()))?;
	if !status.is_success() {
		// Truncated: this ends up in a dialog next to a sentence a person has
		// to read.
		let excerpt: String = body.chars().take(200).collect();
		return Err(status_error(status, &excerpt));
	}
	Ok(body)
}

/// The tag of the newest published release: where `/releases/latest`
/// redirects to. A repository with no release redirects to `/releases`.
async fn latest_tag(
	http: &reqwest::Client,
	origin: &str,
	repo: &str,
) -> Result<String, ResolveError> {
	let response = get_page(http, &format!("{origin}/{repo}/releases/latest")).await?;
	let status = response.status();
	if !status.is_redirection() {
		let err = match page_body(response).await {
			Err(err) => err,
			Ok(_) => ResolveError::new(
				KIND_MALFORMED,
				"the latest release did not redirect",
			),
		};
		return Err(err);
	}
	response.headers()
		.get(reqwest::header::LOCATION)
		.and_then(|location| location.to_str().ok())
		.and_then(|location| location.split_once("/releases/tag/"))
		.map(|(_, tag)| percent_decode(tag))
		.filter(|tag| !tag.is_empty())
		.ok_or_else(|| {
			ResolveError::new(
				KIND_NO_RELEASE,
				"the repository has no published release",
			)
		})
}

/// Undoes the `%XX` escapes of a URL path segment.
fn percent_decode(segment: &str) -> String {
	let bytes = segment.as_bytes();
	let mut out = Vec::with_capacity(bytes.len());
	let mut i = 0;
	while i < bytes.len() {
		let escaped = (bytes[i] == b'%' && i + 2 < bytes.len())
			.then(|| {
				segment.get(i + 1..i + 3)
					.and_then(|hex| u8::from_str_radix(hex, 16).ok())
			})
			.flatten();
		match escaped {
			Some(byte) => {
				out.push(byte);
				i += 3;
			}
			None => {
				out.push(bytes[i]);
				i += 1;
			}
		}
	}
	String::from_utf8_lossy(&out).into_owned()
}

/// The `sha256:<hex>` closest to the end of `text`, if any.
fn last_digest(text: &str) -> Option<String> {
	let mut found = None;
	let mut from = 0;
	while let Some(at) = text[from..].find("sha256:") {
		let start = from + at;
		let hex = text.get(start + 7..start + 71);
		if hex.is_some_and(|h| h.bytes().all(|b| b.is_ascii_hexdigit())) {
			found = Some(text[start..start + 71].to_string());
		}
		from = start + 7;
	}
	found
}

/// The assets an `expanded_assets` page lists, each with the digest shown
/// beside it. Source archives are not assets.
fn parse_assets(page: &str, origin: &str) -> Result<Vec<ReleaseAsset>, ResolveError> {
	if !page.contains("<ul") {
		return Err(ResolveError::new(
			KIND_MALFORMED,
			"not a release assets page",
		));
	}
	let mut items: Vec<&str> = Vec::new();
	let mut rest = page;
	while let Some(at) = rest.find("<li") {
		rest = &rest[at + 3..];
		let opens = rest.starts_with('>') || rest.starts_with(char::is_whitespace);
		let end = rest.find("<li").unwrap_or(rest.len());
		if opens {
			items.push(&rest[..end]);
		}
	}

	let mut assets = Vec::new();
	for item in items {
		let Some(href) = item
			.split_once("href=\"")
			.and_then(|(_, after)| after.split_once('"'))
			.map(|(href, _)| href)
		else {
			continue;
		};
		if href.contains("/archive/") {
			continue;
		}
		let name = percent_decode(href.rsplit('/').next().unwrap_or(href));
		let url = if href.starts_with('/') {
			format!("{origin}{href}")
		} else {
			href.to_string()
		};
		assets.push(ReleaseAsset {
			name,
			browser_download_url: url,
			digest: last_digest(item),
		});
	}
	Ok(assets)
}

/// The asset's checksum, from the strongest source the release offers.
///
/// Two sources, in order:
///
///   1. the digest the release page shows beside the asset, which is GitHub's
///      record of what it stored and needs no extra request;
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
	asset: &ReleaseAsset,
	assets: &[ReleaseAsset],
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

/// Resolves this machine's asset out of `repo`'s release `tag`, or its newest
/// release, against `origin`.
pub async fn resolve(
	origin: &str,
	repo: &str,
	tag: Option<&str>,
	os: &str,
	arch: &str,
) -> Result<ResolvedAsset, ResolveError> {
	let http = client()?;
	let tag = match tag {
		Some(tag) => tag.to_string(),
		None => latest_tag(&http, origin, repo).await?,
	};
	let page = get_page(
		&http,
		&format!("{origin}/{repo}/releases/expanded_assets/{tag}"),
	)
	.await?;
	let assets = parse_assets(&page_body(page).await?, origin)?;
	let asset = select_asset(&assets, os, arch)?;
	let checksum = resolve_checksum(&http, asset, &assets).await;

	Ok(ResolvedAsset {
		tag,
		name: asset.name.clone(),
		url: asset.browser_download_url.clone(),
		checksum,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use wiremock::matchers::{method, path};
	use wiremock::{Mock, MockServer, ResponseTemplate};

	fn asset(name: &str, digest: Option<&str>) -> ReleaseAsset {
		ReleaseAsset {
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
	fn full_release() -> Vec<ReleaseAsset> {
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

	// ── the release page ─────────────────────────────────────────────────

	const REPO_PATH: &str = "/rabbytesoftware/quiver.desktop";

	/// An `expanded_assets` page: one list item per asset, the digest as text
	/// beside the link, and the two source archives GitHub always adds.
	fn page(tag: &str, assets: &[(&str, Option<&str>)]) -> String {
		let mut html = String::from("<div><ul class=\"list-style-none\">");
		for (name, digest) in assets {
			html.push_str(&format!(
				"<li class=\"Box-row\"><a href=\"{REPO_PATH}/releases/download/{tag}/{name}\" rel=\"nofollow\"><span>{name}</span></a>{}</li>",
				digest.map_or(String::new(), |d| format!("<span class=\"digest\">sha256:{d}</span>"))
			));
		}
		html.push_str(&format!(
			"<li><a href=\"{REPO_PATH}/archive/refs/tags/{tag}.zip\">Source code (zip)</a></li></ul></div>"
		));
		html
	}

	async fn serve(tag: &str, html: String) -> MockServer {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(format!("{REPO_PATH}/releases/expanded_assets/{tag}")))
			.respond_with(ResponseTemplate::new(200).set_body_string(html))
			.mount(&server)
			.await;
		server
	}

	async fn latest_redirects_to(server: &MockServer, location: &str) {
		Mock::given(method("GET"))
			.and(path(format!("{REPO_PATH}/releases/latest")))
			.respond_with(
				ResponseTemplate::new(302).insert_header("location", location),
			)
			.mount(server)
			.await;
	}

	#[tokio::test]
	async fn the_normal_case_resolves_a_url_and_a_checksum() {
		let server = serve(
			"stable-26.9",
			page(
				"stable-26.9",
				&[("quiver-desktop_0.1.0_amd64.AppImage", Some(SUM_A))],
			),
		)
		.await;
		latest_redirects_to(&server, &format!("{REPO_PATH}/releases/tag/stable-26.9"))
			.await;

		let resolved = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap();

		assert_eq!(resolved.tag, "stable-26.9");
		assert_eq!(resolved.name, "quiver-desktop_0.1.0_amd64.AppImage");
		assert_eq!(
			resolved.url,
			format!(
				"{}{REPO_PATH}/releases/download/stable-26.9/quiver-desktop_0.1.0_amd64.AppImage",
				server.uri()
			)
		);
		assert_eq!(resolved.checksum.as_deref(), Some(SUM_A));
	}

	#[tokio::test]
	async fn an_explicit_tag_never_asks_which_release_is_the_latest() {
		let server = serve(
			"nightly-rolling",
			page("nightly-rolling", &[("Quiver.AppImage", Some(SUM_A))]),
		)
		.await;

		let resolved = resolve(
			&server.uri(),
			DEFAULT_REPO,
			Some("nightly-rolling"),
			"linux",
			"x86_64",
		)
		.await
		.unwrap();

		assert_eq!(resolved.tag, "nightly-rolling");
		assert_eq!(server.received_requests().await.unwrap().len(), 1);
	}

	/// An asset without a digest on the page still verifies, off the manifest
	/// `install.sh` has always read.
	#[tokio::test]
	async fn a_digestless_asset_falls_back_to_the_published_manifest() {
		let server = serve(
			"t",
			page(
				"t",
				&[("Quiver.AppImage", None), ("SHA256SUMS", Some(SUM_A))],
			),
		)
		.await;
		Mock::given(method("GET"))
			.and(path(format!("{REPO_PATH}/releases/download/t/SHA256SUMS")))
			.respond_with(
				ResponseTemplate::new(200)
					.set_body_string(format!("{SUM_B}  Quiver.AppImage\n")),
			)
			.mount(&server)
			.await;

		let resolved = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
			.await
			.unwrap();
		assert_eq!(resolved.checksum.as_deref(), Some(SUM_B));
	}

	/// Resolution SUCCEEDS with no checksum; refusing to install is the
	/// caller's decision, not this function's.
	#[tokio::test]
	async fn a_release_with_no_checksum_anywhere_resolves_without_one() {
		let server = serve("t", page("t", &[("Quiver.AppImage", None)])).await;

		let resolved = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
			.await
			.unwrap();
		assert_eq!(resolved.checksum, None);
	}

	/// A manifest that is published but 404s, or carries no line for this
	/// asset, is the same as no manifest -- never a hard failure, and never a
	/// checksum invented to fill the hole.
	#[tokio::test]
	async fn an_unreadable_checksum_manifest_is_the_same_as_none() {
		let server = serve(
			"t",
			page("t", &[("Quiver.AppImage", None), ("checksums.txt", None)]),
		)
		.await;
		Mock::given(method("GET"))
			.and(path(format!(
				"{REPO_PATH}/releases/download/t/checksums.txt"
			)))
			.respond_with(ResponseTemplate::new(404))
			.mount(&server)
			.await;

		let resolved = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
			.await
			.unwrap();
		assert_eq!(resolved.checksum, None);
	}

	#[tokio::test]
	async fn no_asset_for_this_platform_is_reported_as_such() {
		let server = serve(
			"t",
			page("t", &[("quiver-desktop_0.1.0_amd64.deb", Some(SUM_A))]),
		)
		.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_NO_ASSET);
	}

	/// A repository whose only releases are pre-releases redirects `latest` to
	/// the releases list, which is no release to install.
	#[tokio::test]
	async fn a_repository_with_no_release_is_distinguished_from_a_dead_network() {
		let server = MockServer::start().await;
		latest_redirects_to(&server, &format!("{REPO_PATH}/releases")).await;

		let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_NO_RELEASE);
	}

	#[tokio::test]
	async fn a_page_that_is_not_a_release_page_is_reported_as_malformed() {
		let server = serve("t", "<html>nope</html>".to_string()).await;

		let err = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_MALFORMED);
	}

	#[tokio::test]
	async fn a_latest_that_does_not_redirect_is_reported_as_malformed() {
		let server = MockServer::start().await;
		Mock::given(method("GET"))
			.and(path(format!("{REPO_PATH}/releases/latest")))
			.respond_with(ResponseTemplate::new(200).set_body_string("<html></html>"))
			.mount(&server)
			.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_MALFORMED);
	}

	#[tokio::test]
	async fn a_throttled_release_page_is_distinguished_from_everything_else() {
		for status in [403, 429] {
			let server = MockServer::start().await;
			Mock::given(method("GET"))
				.respond_with(
					ResponseTemplate::new(status).set_body_string("slow down"),
				)
				.mount(&server)
				.await;

			let err =
				resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
					.await
					.unwrap_err();
			assert_eq!(err.kind, KIND_RATE_LIMITED, "status {status}");
		}
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
			.respond_with(
				ResponseTemplate::new(500).set_body_string("upstream is unwell"),
			)
			.mount(&server)
			.await;

		let err = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
			.await
			.unwrap_err();
		assert_eq!(err.kind, KIND_HTTP);
	}

	#[tokio::test]
	async fn an_unknown_tag_is_a_missing_release() {
		let server = MockServer::start().await;

		let err = resolve(&server.uri(), DEFAULT_REPO, Some("nope"), "linux", "x86_64")
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
	async fn an_unreachable_origin_reports_offline() {
		let err = resolve(
			"http://127.0.0.1:1",
			DEFAULT_REPO,
			Some("t"),
			"linux",
			"x86_64",
		)
		.await
		.unwrap_err();
		assert_eq!(err.kind, KIND_OFFLINE);
	}

	// ── reading the page ─────────────────────────────────────────────────

	#[test]
	fn source_archives_are_not_assets_and_names_are_unescaped() {
		let html = page("t", &[("Quiver%20App.dmg", Some(SUM_A))]);

		let assets = parse_assets(&html, "https://github.com").unwrap();

		assert_eq!(assets.len(), 1);
		assert_eq!(assets[0].name, "Quiver App.dmg");
		assert_eq!(
			assets[0].digest.as_deref(),
			Some(format!("sha256:{SUM_A}").as_str())
		);
	}

	#[test]
	fn the_digest_nearest_the_end_of_an_item_is_the_asset_s() {
		let text = format!("sha256:{SUM_A} then sha256:{SUM_B}");
		assert_eq!(
			last_digest(&text).as_deref(),
			Some(format!("sha256:{SUM_B}").as_str())
		);
		assert_eq!(last_digest("sha256:short"), None);
	}

	#[test]
	fn a_stray_percent_sign_is_left_alone() {
		assert_eq!(percent_decode("100%"), "100%");
		assert_eq!(percent_decode("a%zzb"), "a%zzb");
		assert_eq!(percent_decode("%é"), "%é");
	}
}
