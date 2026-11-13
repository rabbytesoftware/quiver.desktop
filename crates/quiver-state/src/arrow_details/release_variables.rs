//! Quiver's own row installs a release asset nothing in its manifest can name, so the app resolves
//! it at click time. The network part lives in `quiver-release`; this holds the decisions.

use crate::i18n::{en::arrow, Key};
use crate::namespace::bare_namespace;

/// The two variables `ARROW.md` declares without defaults.
pub const RELEASE_ASSET_URL: &str = "QUIVER_RELEASE_ASSET_URL";
pub const RELEASE_CHECKSUM: &str = "QUIVER_RELEASE_CHECKSUM";
pub const RELEASE_VARIABLE_NAMES: [&str; 2] = [RELEASE_ASSET_URL, RELEASE_CHECKSUM];

pub const QUIVER_DESKTOP_NAMESPACE: &str = "github.com/rabbytesoftware/quiver.desktop";

/// Whether this arrow is Quiver itself. The resolver reads quiver.desktop's OWN releases, so it is
/// only ever correct for quiver.desktop's own row, whichever selector that row follows.
pub fn is_self_arrow(namespace: &str) -> bool {
    bare_namespace(namespace) == QUIVER_DESKTOP_NAMESPACE
}

/// The release an install of this row puts on disk: the ref it resolved to. Never the identity
/// selector (`stable` or `v1.*` name what the row follows, not a release).
pub fn install_tag_for(resolved_ref: &str) -> Option<&str> {
    (!resolved_ref.is_empty()).then_some(resolved_ref)
}

/// Why resolving the release asset failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReleaseResolveErrorKind {
    Offline,
    RateLimited,
    NoRelease,
    Http,
    Malformed,
    NoAsset,
    UnsupportedPlatform,
    Unverifiable,
    NothingAhead,
}

impl ReleaseResolveErrorKind {
    /// The wire kind (`parse`'s inverse).
    pub fn wire(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::RateLimited => "rate_limited",
            Self::NoRelease => "no_release",
            Self::Http => "http",
            Self::Malformed => "malformed",
            Self::NoAsset => "no_asset",
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::Unverifiable => "unverifiable",
            Self::NothingAhead => "nothing_ahead",
        }
    }

    /// Parses the wire kind.
    pub fn parse(kind: &str) -> Option<Self> {
        Some(match kind {
            "offline" => Self::Offline,
            "rate_limited" => Self::RateLimited,
            "no_release" => Self::NoRelease,
            "http" => Self::Http,
            "malformed" => Self::Malformed,
            "no_asset" => Self::NoAsset,
            "unsupported_platform" => Self::UnsupportedPlatform,
            "unverifiable" => Self::Unverifiable,
            "nothing_ahead" => Self::NothingAhead,
            _ => return None,
        })
    }
}

/// The non-technical sentence for a resolution failure. An unknown `kind` (a newer Rust side talking
/// to an older bundle) falls back to the generic one rather than rendering a raw key.
pub fn release_error_message_key(kind: &str) -> Key {
    match ReleaseResolveErrorKind::parse(kind) {
        Some(ReleaseResolveErrorKind::Offline) => arrow::RELEASE_OFFLINE,
        Some(ReleaseResolveErrorKind::RateLimited) => arrow::RELEASE_RATE_LIMITED,
        Some(ReleaseResolveErrorKind::NoRelease) => arrow::RELEASE_NO_RELEASE,
        Some(ReleaseResolveErrorKind::NoAsset) => arrow::RELEASE_NO_ASSET,
        Some(ReleaseResolveErrorKind::UnsupportedPlatform) => arrow::RELEASE_UNSUPPORTED_PLATFORM,
        Some(ReleaseResolveErrorKind::Unverifiable) => arrow::RELEASE_UNVERIFIABLE,
        Some(ReleaseResolveErrorKind::NothingAhead) => arrow::RELEASE_NOTHING_AHEAD,
        Some(ReleaseResolveErrorKind::Http | ReleaseResolveErrorKind::Malformed) | None => {
            arrow::RELEASE_UNAVAILABLE
        }
    }
}

/// The two release variables from a resolved asset, or the `unverifiable` failure when there is no
/// published checksum. It does not inherit `install.sh`'s "warn and install anyway": the manifest
/// verifies the checksum variable and core refuses an empty one, so offering it would fail mid-update
/// with the app already killed.
pub fn release_variables(
    asset_name: &str,
    asset_tag: &str,
    url: &str,
    checksum: &str,
) -> Result<Vec<(String, String)>, ReleaseFailure> {
    if checksum.is_empty() {
        return Err(ReleaseFailure {
            kind: ReleaseResolveErrorKind::Unverifiable,
            detail: format!(
                "{asset_name} ({asset_tag}) has no published sha256 digest or checksum manifest"
            ),
        });
    }
    Ok(vec![
        (RELEASE_ASSET_URL.into(), url.into()),
        (RELEASE_CHECKSUM.into(), checksum.into()),
    ])
}

/// What updating needs: the release the row moves to. With nothing newer the update is refused (falling
/// back to the installed ref would reinstall the old release while core advances the row).
pub fn update_target(
    namespace: &str,
    available_ref: Option<&str>,
) -> Result<String, ReleaseFailure> {
    available_ref
        .filter(|r| !r.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ReleaseFailure {
            kind: ReleaseResolveErrorKind::NothingAhead,
            detail: format!("{namespace} has nothing newer to update to"),
        })
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReleaseFailure {
    pub kind: ReleaseResolveErrorKind,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_kinds_round_trip() {
        for kind in [
            "offline",
            "rate_limited",
            "no_release",
            "http",
            "malformed",
            "no_asset",
            "unsupported_platform",
            "unverifiable",
            "nothing_ahead",
        ] {
            assert_eq!(ReleaseResolveErrorKind::parse(kind).unwrap().wire(), kind);
        }
    }

    #[test]
    fn self_arrow_ignores_the_selector() {
        assert!(is_self_arrow("github.com/rabbytesoftware/quiver.desktop"));
        assert!(is_self_arrow(
            "github.com/rabbytesoftware/quiver.desktop@stable"
        ));
        assert!(!is_self_arrow(
            "github.com/rabbytesoftware/quiver.core@stable"
        ));
        assert!(!is_self_arrow("github.com/rabbyte/caddy"));
    }

    #[test]
    fn install_tag_is_the_resolved_ref_or_nothing() {
        assert_eq!(install_tag_for("v1.2.0"), Some("v1.2.0"));
        assert_eq!(install_tag_for(""), None);
    }

    #[test]
    fn every_error_kind_has_a_sentence_and_unknown_ones_fall_back() {
        assert_eq!(release_error_message_key("offline"), arrow::RELEASE_OFFLINE);
        assert_eq!(
            release_error_message_key("rate_limited"),
            arrow::RELEASE_RATE_LIMITED
        );
        assert_eq!(
            release_error_message_key("http"),
            arrow::RELEASE_UNAVAILABLE
        );
        assert_eq!(
            release_error_message_key("malformed"),
            arrow::RELEASE_UNAVAILABLE
        );
        assert_eq!(
            release_error_message_key("from-the-future"),
            arrow::RELEASE_UNAVAILABLE
        );
    }

    #[test]
    fn a_release_without_a_checksum_is_unverifiable() {
        let err = release_variables("quiver.tgz", "v1", "https://x/q.tgz", "").unwrap_err();
        assert_eq!(err.kind, ReleaseResolveErrorKind::Unverifiable);
        assert!(err.detail.contains("quiver.tgz (v1)"));
        let ok = release_variables("q", "v1", "https://x", "abc").unwrap();
        assert_eq!(
            ok[0],
            (RELEASE_ASSET_URL.to_owned(), "https://x".to_owned())
        );
        assert_eq!(ok[1], (RELEASE_CHECKSUM.to_owned(), "abc".to_owned()));
    }

    #[test]
    fn updating_with_nothing_ahead_is_refused() {
        let err = update_target("ns", None).unwrap_err();
        assert_eq!(err.kind, ReleaseResolveErrorKind::NothingAhead);
        assert_eq!(update_target("ns", Some("v2")).unwrap(), "v2");
    }
}
