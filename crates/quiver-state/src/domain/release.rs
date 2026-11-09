use serde::{Deserialize, Serialize};

/// Quiver's own arrow namespace, refless: the one whose release assets this app can resolve, and
/// the one it announces itself under on every daemon connection.
pub const QUIVER_DESKTOP_NAMESPACE: &str = "github.com/rabbytesoftware/quiver.desktop";

/// quiver.core's own arrow namespace, refless -- the daemon self-registers under this exactly like
/// any other catalogued arrow, which is what lets its self-update channel be read via the ordinary
/// `GET /v0/arrow/:ns/channels`.
pub const QUIVER_CORE_NAMESPACE: &str = "github.com/rabbytesoftware/quiver.core";

/// Whether `namespace` is one of Quiver's own two self-registered components. Refs are stripped
/// since a catalog entry's namespace carries one while a search result's does not.
///
/// Deliberately separate from the self-update `is_self_arrow` of arrow-details: that one must never
/// also match quiver.core. This is for the unrelated "hide Quiver's own rows from listings" preference.
pub fn is_quiver_own_component(namespace: &str) -> bool {
    let bare = crate::namespace::bare_namespace(namespace);
    bare == QUIVER_DESKTOP_NAMESPACE || bare == QUIVER_CORE_NAMESPACE
}

/// This app's own release asset, as the release resolver finds it.
///
/// Quiver's own `ARROW.md` declares `QUIVER_RELEASE_ASSET_URL` and `QUIVER_RELEASE_CHECKSUM` with no
/// defaults, which quiver.core reads as "required from the caller". Nothing in the manifest can fill
/// them (release filenames carry a static version rather than the git tag, and `${REF}` during an
/// update is the ref being updated FROM), so the caller resolves the asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedReleaseAsset {
    /// The release tag the asset came from.
    pub tag: String,
    /// The asset's filename.
    pub name: String,
    /// `QUIVER_RELEASE_ASSET_URL`.
    pub url: String,
    /// `QUIVER_RELEASE_CHECKSUM`: bare lowercase hex. `None` when the release publishes neither a
    /// per-asset digest nor a checksum manifest -- a fact about the release, not a verdict.
    pub checksum: Option<String>,
}

/// Why resolving failed. The frontend branches on `kind` to pick a sentence; `detail` is wire text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseResolveErrorKind {
    /// DNS, TLS, connect or timeout -- the machine could not reach GitHub.
    Offline,
    /// GitHub refused on quota. Unauthenticated callers share 60 requests an hour per IP.
    RateLimited,
    /// The repository publishes no release the API will name.
    NoRelease,
    /// Any other non-success status.
    Http,
    /// A 2xx whose body was not a release document.
    Malformed,
    /// The release carries nothing installable on this OS and architecture.
    NoAsset,
    /// This build is running somewhere no Quiver bundle is published for.
    UnsupportedPlatform,
    /// Raised on this side: the asset resolved, but nothing published a checksum for it. Whether
    /// that blocks an install is a policy decision rather than a resolution failure.
    Unverifiable,
    /// Raised on this side: core reports nothing newer for the row an update was asked of.
    NothingAhead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseResolveError {
    pub kind: ReleaseResolveErrorKind,
    pub detail: String,
}

/// Whether an unknown JSON value from the boundary is one of these errors.
pub fn is_release_resolve_error(value: &serde_json::Value) -> bool {
    serde_json::from_value::<ReleaseResolveError>(value.clone()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_component_matches_bare_and_with_ref() {
        assert!(is_quiver_own_component(QUIVER_DESKTOP_NAMESPACE));
        assert!(is_quiver_own_component(&format!("{QUIVER_DESKTOP_NAMESPACE}@stable-1.0")));
        assert!(is_quiver_own_component(QUIVER_CORE_NAMESPACE));
        assert!(is_quiver_own_component(&format!("{QUIVER_CORE_NAMESPACE}@v1.0.0")));
    }

    #[test]
    fn own_component_rejects_unrelated_and_substring_matches() {
        assert!(!is_quiver_own_component("github.com/rabbyte/minecraft@v1.21.4"));
        assert!(!is_quiver_own_component("github.com/rabbytesoftware/quiver.core-extra"));
    }

    #[test]
    fn release_error_guard() {
        assert!(is_release_resolve_error(&serde_json::json!({"kind":"offline","detail":"x"})));
        assert!(!is_release_resolve_error(&serde_json::json!({"kind":5})));
        assert!(!is_release_resolve_error(&serde_json::json!(null)));
    }
}
