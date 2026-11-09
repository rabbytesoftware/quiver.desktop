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
//! Done natively, not in the UI layer, because selection needs the real
//! OS/CPU architecture and a webview cannot report either honestly (Apple
//! Silicon's `navigator.platform` still answers `MacIntel`).
//! `std::env::consts` is the direct analogue of `install.sh`'s
//! `uname -s`/`uname -m`.
//!
//! Selection rules are a port of `install.sh`'s, not a second opinion.
//!
//! Nothing here calls `api.github.com`. Its anonymous quota is 60 requests an
//! hour per IP, which a shared NAT exhausts without this user having made one,
//! and the update then refuses to start. The release page does not count
//! against it: the tag is the redirect of `/releases/latest`, and the assets
//! and their sha256 digests are on `/releases/expanded_assets/<tag>`.

use crate::asset::ResolvedAsset;
use crate::checksum::resolve_checksum;
use crate::http::{client, get_page, latest_tag, page_body};
use crate::page::parse_assets;
use crate::select::select_asset;
use crate::Result;

/// The repository this app publishes itself from.
pub const DEFAULT_REPO: &str = "rabbytesoftware/quiver.desktop";

/// GitHub's origin. Overridable only so the tests can point at a local
/// server; nothing reads an environment variable for it at runtime.
pub const DEFAULT_ORIGIN: &str = "https://github.com";

/// Resolves this machine's asset out of `repo`'s release `tag`, or its newest
/// release, against `origin`.
pub async fn resolve(
    origin: &str,
    repo: &str,
    tag: Option<&str>,
    os: &str,
    arch: &str,
) -> Result<ResolvedAsset> {
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

/// This app's own release asset for the platform it is running on, from the
/// release tagged `tag` -- the ref the row is moving to -- or the newest
/// release when there is none.
///
/// Called at the moment the user clicks Update (or Install) on Quiver's own
/// tile, to fill in `QUIVER_RELEASE_ASSET_URL` and `QUIVER_RELEASE_CHECKSUM`.
///
/// `std::env::consts` and not a runtime probe: these are the compiler's own
/// answer for the binary being executed, so a universal macOS bundle running
/// under Rosetta still reports the architecture it was BUILT for, which is
/// the one whose bundle it should be updated with.
///
/// Supplying the GitHub defaults is this function's job rather than
/// [`resolve`]'s: a function that reaches for `github.com` with no way to be
/// pointed elsewhere cannot be tested without the network.
pub async fn resolve_own_release(tag: Option<&str>) -> Result<ResolvedAsset> {
    resolve(
        DEFAULT_ORIGIN,
        DEFAULT_REPO,
        tag,
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
    .await
}
