use serde::Serialize;

/// One asset as the release page lists it.
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    /// `"sha256:<hex>"` when the page shows one for this asset, `None`
    /// otherwise. GitHub began recording it in 2025, so an older release
    /// carries none, which is why the resolver still falls back to a
    /// published checksum manifest.
    pub digest: Option<String>,
}

/// What the UI gets back: enough to start an execution with.
///
/// `checksum` is an `Option` on purpose. This type reports what the release
/// actually publishes; whether an unverifiable asset may be installed is a
/// policy decision, and it is made at the call site (see
/// `src/features/arrow-details/lib/release-variables.ts` in the Tauri app)
/// rather than buried in a resolver that would have to decide it identically
/// for every caller.
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
